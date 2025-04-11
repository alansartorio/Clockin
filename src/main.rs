use std::{
    env::{Args, args},
    io::stdin,
    process,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use summary::Summary;
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
    Edit,
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

            let line_receiver = lines(cancel);
            while let Some(line) = line_receiver.recv().unwrap() {
                writer.write_line(&line)?;
            }

            writer.end()?;
        }
        Command::Summary => {
            let path = file::require_clockin_file()?;
            let sessions = parser::parse_file(path).unwrap();
            let summary = Summary::summarize(sessions);
            for (week, weekdata) in summary.weeks {
                println!(
                    "Week {}: {}",
                    week.first_day(),
                    fmt_duration(&weekdata.duration())
                );

                for (date, duration) in weekdata.days {
                    println!("- {}: {}", date, fmt_duration(&duration));
                }
            }
        }
        _ => unimplemented!("command"),
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
