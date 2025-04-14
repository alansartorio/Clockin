use std::{
    io::stdin,
    os::unix::process::CommandExt,
    process,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, TimeZone, Weekday};
use clap::Parser;
use cli::Command;
use file::get_data_dir;
use summary::{MonthId, NaiveDateExt, Summary};
use writer::Writer;

mod cli;
mod file;
mod parser;
mod summary;
mod writer;

fn get_shell() -> String {
    std::env::var("SHELL").unwrap_or("sh".to_owned())
}

fn lines(cancel: Receiver<()>) -> Receiver<Option<String>> {
    let (sender, receiver) = mpsc::channel();
    let sender2 = sender.clone();

    thread::spawn(move || {
        for line in stdin().lines() {
            let line = line.unwrap();
            sender.send(Some(line)).unwrap();
        }
        sender.send(None).unwrap();
    });
    thread::spawn(move || {
        cancel.recv().unwrap();
        sender2.send(None).unwrap();
    });

    receiver
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

fn fmt_datetime<Tz: TimeZone>(dt: &DateTime<Tz>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
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
        Weekday::Wed => "Miercoles",
        Weekday::Thu => "Jueves",
        Weekday::Fri => "Viernes",
        Weekday::Sat => "Sabado",
        Weekday::Sun => "Domingo",
    }
}

fn run(command: Command, cancel: Receiver<()>) -> Result<()> {
    match command {
        Command::Link { name } => {
            file::create_clockin_file(&name)?;
        }
        Command::Edit => {
            let path = file::require_clockin_file()?;
            let editor = std::env::var("EDITOR").unwrap_or("nano".to_owned());
            let mut process = process::Command::new(editor)
                .arg(path)
                .spawn()
                .context("error while trying to run editor")?;
            process.wait().context("error while editing file")?;
        }
        Command::In => {
            let mut writer = Writer::new(file::require_clockin_file()?)?;
            println!(
                "{}",
                concat!("==============\n", "= CLOCKED IN =\n", "==============")
            );
            println!("{}", fmt_datetime(&writer.start));

            let line_receiver = lines(cancel);
            while let Some(line) = line_receiver.recv().unwrap() {
                writer.write_line(&line)?;
            }

            let end = writer.end()?;
            println!("{}", fmt_datetime(&end));
        }
        Command::Summary => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap();
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
        Command::Binnacle { from, to, timezone } => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap();
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
