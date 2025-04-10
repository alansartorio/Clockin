use std::{
    env::{Args, args, current_dir},
    fs::{self, File},
    io::{Write, stdin},
    os,
    path::PathBuf,
    process,
    str::FromStr,
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::{Context, Result, anyhow};
use chrono::Local;

#[derive(Debug)]
enum Command {
    Link { name: String },
    ClockIn,
    ClockOut,
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
        "out" => Ok(Command::ClockOut),
        "summary" => Ok(Command::Summary),
        "edit" => Ok(Command::Edit),
        command => Err(anyhow!("invalid command {command}")),
    }
}

fn find_clockin_file() -> Option<PathBuf> {
    let first_dir = current_dir().unwrap();
    let mut maybe_dir = Some(first_dir.as_path());

    while let Some(dir) = maybe_dir {
        let mut file = dir.to_owned();
        file.push(".clockin");
        if file.exists() {
            return Some(file);
        }
        maybe_dir = dir.parent();
    }

    None
}

fn get_var_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .map(|home| PathBuf::from_str(home.as_str()).unwrap())
        .ok()
}

fn get_data_dir() -> PathBuf {
    let mut data = get_var_path("XDG_DATA_HOME")
        .or_else(|| {
            get_var_path("HOME").map(|mut home| {
                home.push(".local/share");
                home
            })
        })
        .unwrap();
    data.push("clockin");
    fs::create_dir_all(&data).unwrap();
    data
}

fn create_clockin_file(name: &str) -> Result<PathBuf> {
    let mut data = get_data_dir();
    data.push(name);
    let clockin_link = PathBuf::from_str(".clockin").unwrap();
    File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&data)?;
    os::unix::fs::symlink(&data, &clockin_link)?;
    Ok(clockin_link)
}

fn require_clockin_file() -> Result<PathBuf> {
    find_clockin_file().ok_or(anyhow!(".clockin file not found"))
}

fn now_string() -> String {
    let now = Local::now();
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn lines(cancel: Receiver<()>) -> Receiver<Option<String>> {
    let (sender, receiver) = mpsc::channel();
    let sender2 = sender.clone();

    thread::spawn(move || {
        for line in stdin().lines() {
            let line = line.unwrap();
            sender.send(Some(line)).unwrap();
        }
    });
    thread::spawn(move || {
        cancel.recv().unwrap();
        sender2.send(None).unwrap();
    });

    receiver
}

fn run(command: Command, cancel: Receiver<()>) -> Result<()> {
    match &command {
        Command::Link { name } => {
            create_clockin_file(name)?;
        }
        Command::Edit => {
            let path = require_clockin_file()?;
            let editor = std::env::var("EDITOR").unwrap_or("nano".to_owned());
            let mut process = process::Command::new(editor)
                .arg(path)
                .spawn()
                .context("error while trying to run editor")?;
            process.wait().context("error while editing file")?;
        }
        Command::ClockIn => {
            let mut file = File::options()
                .append(true)
                .open(require_clockin_file()?)
                .context("opening clockin file")?;
            file.write_all(format!("-{}\n", now_string()).as_bytes())
                .context("writing start time")?;

            let line_receiver = lines(cancel);
            while let Some(line) = line_receiver.recv().unwrap() {
                file.write_all(line.as_bytes())
                    .context("writing description")?;
                file.write_all(b"\n")?;
            }

            file.write_all(format!("+{}\n", now_string()).as_bytes())
                .context("writing end time")?;
        }
        _ => unimplemented!("command"),
    };

    println!("command: {command:?}");
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
