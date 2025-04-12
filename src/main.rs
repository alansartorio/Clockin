use std::{
    env::{Args, args},
    io::stdin,
    os::unix::process::CommandExt,
    process,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Datelike, TimeZone, Weekday};
use file::get_data_dir;
use summary::{MonthId, NaiveDateExt, Summary};
use writer::Writer;

mod file;
mod parser;
mod summary;
mod writer;

#[derive(Debug)]
enum Command {
    Link { name: String },
    ClockIn,
    Summary,
    Binnacle,
    Edit,
    Cd,
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
        "binnacle" | "bitacora" => Ok(Command::Binnacle),
        "cd" => Ok(Command::Cd),
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
    let minutes = duration / 60 - hours * 60;
    let seconds = duration - minutes * 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
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
    match &command {
        Command::Link { name } => {
            file::create_clockin_file(name)?;
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
            let summary = Summary::summarize(sessions);

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
        Command::Binnacle => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap();
            let summary = Summary::summarize(sessions);

            let mut last_month = None;
            for (date, day) in &summary.days {
                let month = date.month_id();

                if last_month.is_none_or(|last_month| last_month != month) {
                    last_month = Some(month);
                    println!("## {}\n", fmt_month(month));
                }

                println!(
                    "- {} {:02}/{:02} ({})\n",
                    fmt_weekday(date.weekday()),
                    date.day0() + 1,
                    date.month0() + 1,
                    fmt_duration(&day.duration)
                );
                for description in &day.descriptions {
                    println!("\t- {}\n", description);
                }
            }
        }
        Command::Cd => {
            let shell = std::env::var("SHELL").unwrap_or("sh".to_owned());
            Err(process::Command::new(&shell)
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
