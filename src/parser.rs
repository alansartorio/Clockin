use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::Path,
};

use anyhow::Result;
use chrono::{DateTime, Duration, FixedOffset, Local};

type DT = DateTime<FixedOffset>;

#[derive(Debug)]
pub struct Session {
    pub start: DT,
    pub end: DT,
    pub description: String,
}
impl Session {
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
}

pub struct SessionIterator {
    lines: Lines<BufReader<File>>,
}

fn is_macro_line(line: impl AsRef<str>, prefix: char) -> bool {
    line.as_ref().chars().nth(0) == Some('%') && line.as_ref().chars().nth(1) == Some(prefix)
}

fn extract_macro(line: &str, prefix: char) -> Option<DateTime<FixedOffset>> {
    is_macro_line(line, prefix)
        .then(|| &line[2..])
        .map(|d| DateTime::parse_from_rfc3339(d).unwrap())
}

impl Iterator for SessionIterator {
    type Item = Session;

    fn next(&mut self) -> Option<Self::Item> {
        let start = 'a: {
            loop {
                let line = self.lines.next()?.unwrap();
                if let Some(m) = extract_macro(&line, '-') {
                    break 'a m;
                }
            }
        };

        let mut description = String::new();
        let mut end = Local::now().fixed_offset();

        loop {
            let Some(line) = self.lines.next() else {
                break;
            };
            let line = line.unwrap();
            if let Some(m) = extract_macro(&line, '+') {
                end = m;
                break;
            } else {
                description.push_str(&line);
                description.push('\n');
            }
        }

        Some(Session {
            start,
            end,
            description: description.trim_end().to_owned(),
        })
    }
}

pub fn parse_file(path: impl AsRef<Path>) -> Result<SessionIterator> {
    let file = BufReader::new(File::open(path)?);
    Ok(SessionIterator {
        lines: file.lines(),
    })
}
