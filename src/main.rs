use std::{
    env::{Args, args},
    io::stdin,
    ops::Bound,
    os::unix::process::CommandExt,
    process,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, TimeZone, Weekday};
use file::get_data_dir;
use summary::{MonthId, NaiveDateExt, Summary};
use writer::Writer;

mod file;
mod parser;
mod summary;
mod writer;

#[derive(Debug)]
enum Command {
    Link {
        name: String,
    },
    ClockIn,
    Summary,
    Binnacle {
        range: (Bound<NaiveDate>, Bound<NaiveDate>),
        timezone: FixedOffset,
    },
    Edit,
    Cd,
    Exec {
        command: String,
    },
}

fn get_shell() -> String {
    std::env::var("SHELL").unwrap_or("sh".to_owned())
}

fn parse_binnacle_args<'a>(mut args: impl Iterator<Item = &'a str>) -> Result<Command> {
    let mut from = Bound::Unbounded;
    let mut to = Bound::Unbounded;
    let mut timezone = None;
    while let Some(arg) = args.next() {
        match arg {
            "--from" | "-f" => {
                from = Bound::Included(
                    NaiveDate::parse_from_str(
                        args.next()
                            .ok_or_else(|| anyhow!("expected argument value after \"--from\""))?,
                        "%Y-%m-%d",
                    )
                    .context("could not parse \"--from\" date")?,
                )
            }
            "--to" | "-t" => {
                to = Bound::Included(
                    NaiveDate::parse_from_str(
                        args.next()
                            .ok_or_else(|| anyhow!("expected argument value after \"--to\""))?,
                        "%Y-%m-%d",
                    )
                    .context("could not parse \"--to\" date")?,
                )
            }
            "--timezone" | "-tz" => {
                timezone = Some(
                    args.next()
                        .ok_or_else(|| anyhow!("expected argument value after \"--timezone\""))?
                        .parse::<FixedOffset>()?,
                )
            }
            arg => Err(anyhow!("unrecognized argument \"{arg}\""))?,
        }
    }
    Ok(Command::Binnacle {
        range: (from, to),
        timezone: timezone.unwrap_or_else(|| Local::now().fixed_offset().timezone()),
    })
}

fn parse_args(args: Args) -> Result<Command> {
    let args: Vec<String> = args.skip(1).collect();

    match args
        .first()
        .expect("missing command")
        .to_lowercase()
        .as_str()
    {
        "link" => Ok(Command::Link {
            name: args
                .get(1)
                .ok_or(anyhow!("missing \"link name\" argument"))?
                .to_owned(),
        }),
        "in" => Ok(Command::ClockIn),
        "summary" => Ok(Command::Summary),
        "edit" => Ok(Command::Edit),
        "binnacle" | "bitacora" => parse_binnacle_args(args.iter().map(String::as_str).skip(1)),
        "cd" => Ok(Command::Cd),
        "exec" => Ok(Command::Exec {
            command: args
                .get(1)
                .ok_or(anyhow!("missing \"command\" argument"))?
                .to_owned(),
        }),
        command => Err(anyhow!("invalid command {command}")),
    }
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
        Command::ClockIn => {
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
        Command::Binnacle { range, timezone } => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap();
            let summary = Summary::summarize(sessions, &timezone);
            let current_date = Local::now().with_timezone(&timezone).date_naive();

            let mut last_month = None;
            for (date, day) in summary.days.range(range) {
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

fn main() {
    let command = parse_args(args()).expect("error while parsing arguments");

    let (canceller, cancel) = mpsc::channel();
    ctrlc::set_handler(move || {
        canceller
            .send(())
            .expect("could not send signal on channel.")
    })
    .expect("error setting Ctrl-C handler");
    run(command, cancel).expect("error while running command");
}
