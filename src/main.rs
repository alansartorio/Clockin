use std::{
    ops::RangeBounds,
    os::unix::process::CommandExt,
    path::Path,
    process,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveTime, TimeDelta, Timelike, Weekday};
use clap::Parser;
use cli::Command;
use file::get_data_dir;
use summary::{MonthId, NaiveDateExt, Summary};
use writer::write_date;

use crate::parser::SessionIteratorClosingExt;

mod cli;
mod file;
mod parser;
mod subscribe;
mod summary;
mod writer;

fn get_shell() -> String {
    std::env::var("SHELL").unwrap_or("sh".to_owned())
}

fn fmt_duration(duration: &Duration) -> String {
    let duration = duration.as_secs();
    let hours = duration / (60 * 60);
    let total_minutes = duration / 60;
    let minutes = total_minutes - hours * 60;
    let seconds = duration - total_minutes * 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn fmt_duration_uncertain(duration: &Duration, completed: bool) -> String {
    let mut out = fmt_duration(duration);
    if !completed {
        out.push_str(" (incompleto)");
    }

    out
}

fn fmt_month(month: MonthId) -> String {
    let month_name = [
        "Enero",
        "Febrero",
        "Marzo",
        "Abril",
        "Mayo",
        "Junio",
        "Julio",
        "Agosto",
        "Septiembre",
        "Octubre",
        "Noviembre",
        "Diciembre",
    ][month.month() as usize];
    format!("{} {}", month_name, month.year())
}

fn fmt_weekday(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Lunes",
        Weekday::Tue => "Martes",
        Weekday::Wed => "Miércoles",
        Weekday::Thu => "Jueves",
        Weekday::Fri => "Viernes",
        Weekday::Sat => "Sabado",
        Weekday::Sun => "Domingo",
    }
}

fn edit_file(path: impl AsRef<Path>) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or("nano".to_owned());
    let mut process = process::Command::new(editor)
        .arg(path.as_ref())
        .spawn()
        .context("error while trying to run editor")?;
    process.wait().context("error while editing file")?;
    Ok(())
}

fn fmt_hours_mins(t: NaiveTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn run(command: Command, cancel: Receiver<()>) -> Result<()> {
    match command {
        Command::Link { name } => {
            file::create_clockin_file(&name)?;
        }
        Command::Edit => {
            let file = file::require_clockin_file()?;
            edit_file(file)?;
        }
        Command::In => {
            println!(
                "{}",
                concat!("==============\n", "= CLOCKED IN =\n", "==============")
            );

            let file = file::require_clockin_file()?;
            write_date(&file, false, '-')?;
            edit_file(&file)?;
            write_date(&file, true, '+')?;
        }
        Command::WeekSummary => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap().as_finished_now();
            let summary = Summary::summarize(sessions, &Local);

            let mut last_week = None;
            for (date, day) in &summary.days {
                let week = date.real_week();

                if last_week.is_none_or(|last_week| last_week != week) {
                    last_week = Some(week);
                    println!(
                        "Week {}: {}",
                        week.first_day(),
                        fmt_duration(&summary.week_duration(week))
                    );
                }

                println!("- {}: {}", date, fmt_duration(&day.duration));
            }
        }
        Command::Summary { from, to, timezone } => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap().as_finished_now();
            let summary = Summary::summarize(sessions, &timezone);
            let current_date = Local::now().with_timezone(&timezone).date_naive();

            let mut last_month = None;
            for (date, day) in summary.days.range((from, to)) {
                let month = date.month_id();

                if last_month.is_none_or(|last_month| last_month != month) {
                    last_month = Some(month);
                    println!(
                        "## {} ({})\n",
                        fmt_month(month),
                        fmt_duration_uncertain(
                            &summary.duration(month.first_day()..=month.last_day()),
                            current_date > month.last_day()
                        )
                    );
                }

                println!(
                    "- {} {:02}/{:02} ({})\n",
                    fmt_weekday(date.weekday()),
                    date.day0() + 1,
                    date.month0() + 1,
                    fmt_duration_uncertain(&day.duration, &current_date > date)
                );
                for description in &day.descriptions {
                    println!("\t- {}\n", description);
                }
            }
        }
        Command::WorkTimeAnalysis { from, to, timezone } => {
            let path = file::require_clockin_file()?;

            const ANALYSIS_INTERVAL: TimeDelta = TimeDelta::minutes(30);
            const SLOTS_PER_DAY: usize =
                (TimeDelta::days(1).num_minutes() / ANALYSIS_INTERVAL.num_minutes()) as usize;
            // one counter every interval
            let mut results = [TimeDelta::zero(); SLOTS_PER_DAY];

            let sessions = parser::parse_file(path)
                .unwrap()
                .as_finished_now()
                .filter(|s| (from, to).contains(&s.start.with_timezone(&timezone).date_naive()))
                .map(|s| s.naive_local())
                .flat_map(|s| s.split_at_days())
                .map(|s| s.start.time()..s.end.time());

            for session in sessions {
                for (i, result) in results.iter_mut().enumerate() {
                    let interval_start = NaiveTime::MIN + ANALYSIS_INTERVAL * (i as i32);
                    let interval_end = interval_start + ANALYSIS_INTERVAL;
                    // this fix is needed because "session end" is exclusive but NaiveTime wraps
                    // around at "24:00:00"
                    let fix_end = |t| {
                        if t == NaiveTime::MIN {
                            NaiveTime::MIN - TimeDelta::nanoseconds(1)
                        } else {
                            t
                        }
                    };
                    let overlap = (fix_end(session.end).min(fix_end(interval_end))
                        - session.start.max(interval_start))
                    .max(TimeDelta::zero());
                    *result += overlap;
                }
            }

            let total: TimeDelta = results.iter().sum();

            for (i, result) in results.iter().enumerate() {
                let interval_start = NaiveTime::MIN + ANALYSIS_INTERVAL * (i as i32);
                let interval_end = interval_start + ANALYSIS_INTERVAL;
                let _total_hours = result.num_seconds() as f64 / 3600f64;
                let percentage = result.num_seconds() as f64 / total.num_seconds() as f64;
                println!(
                    "{}-{}: {:.02}% {}",
                    fmt_hours_mins(interval_start),
                    fmt_hours_mins(interval_end),
                    100.0 * percentage,
                    "#".repeat((800.0 * percentage).round() as usize)
                );
            }
        }
        Command::Subscribe => {
            let path = file::require_clockin_project_file()?;
            subscribe::subscribe(&path, cancel)?;
        }
        Command::GetWorkedTime { specification } => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap().as_finished_now();

            let matching_sessions: Vec<_> = match specification {
                cli::GetWorkedTimeCommand::Today { timezone } => {
                    let today = Local::now().with_timezone(&timezone).date_naive();
                    sessions
                        .filter(|s| s.start.with_timezone(&timezone).date_naive() == today)
                        .collect()
                }
                cli::GetWorkedTimeCommand::ByDateRange { from, to, timezone } => sessions
                    .filter(|s| (from, to).contains(&s.start.with_timezone(&timezone).date_naive()))
                    .collect(),
                cli::GetWorkedTimeCommand::LastSession => sessions.last().into_iter().collect(),
            };

            let worked_time: TimeDelta = matching_sessions.into_iter().map(|s| s.duration()).sum();
            println!("{}", worked_time.as_seconds_f64() as u64);
        }
        Command::Cd => {
            Err(process::Command::new(get_shell())
                .current_dir(get_data_dir())
                .exec())?;
        }
        Command::Exec { command } => {
            Err(process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(get_data_dir())
                .exec())?;
        }
    };

    Ok(())
}

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let command = args.command.unwrap_or(Command::In);

    let (canceller, cancel) = mpsc::channel();
    ctrlc::set_handler(move || {
        canceller
            .send(())
            .expect("could not send signal on channel.")
    })
    .expect("error setting Ctrl-C handler");
    run(command, cancel).context("error while running command")?;
    Ok(())
}
