use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use notify::{
    EventKind, RecursiveMode,
    event::{AccessKind, AccessMode},
};
use notify_debouncer_full::new_debouncer;

use crate::parser;

fn watch_file(path: &PathBuf, mut f: impl FnMut(), cancel: Receiver<()>) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx)?;
    debouncer.watch(
        path.parent().context("unable to find path parent")?,
        RecursiveMode::Recursive,
    )?;

    thread::spawn(move || {
        cancel.recv().unwrap();
        debouncer.stop();
    });

    for res in rx {
        match res {
            Ok(event) => {
                event.iter().for_each(|event| {
                    eprintln!("event: {event:?}");
                });
                event
                    .into_iter()
                    .filter(|e| e.paths.contains(path))
                    .filter(|e| {
                        matches!(
                            e.kind,
                            EventKind::Access(AccessKind::Close(AccessMode::Write))
                        )
                    })
                    .for_each(|_e| f());
            }
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }

    Ok(())
}

enum SessionStatus {
    Finished,
    Started,
}

fn get_last_session_status(path: &PathBuf) -> Result<SessionStatus> {
    let parser = parser::parse_file(path)?;
    let was_last_session_finished = parser.last().map(|s| s.is_finished()).unwrap_or(true);

    Ok(if was_last_session_finished {
        SessionStatus::Finished
    } else {
        SessionStatus::Started
    })
}

fn print_last_session_status(path: &PathBuf) {
    match get_last_session_status(path).unwrap() {
        SessionStatus::Started => println!("started"),
        SessionStatus::Finished => println!("finished"),
    }
}

pub fn subscribe(path: &PathBuf, cancel: Receiver<()>) -> Result<()> {
    print_last_session_status(path);
    watch_file(path, || print_last_session_status(path), cancel)
}
