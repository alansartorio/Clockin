use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::Path,
};

use anyhow::Result;
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDateTime, NaiveTime, TimeZone};

#[derive(Debug, PartialEq)]
pub struct NaiveSession {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub description: String,
}

#[derive(Debug)]
pub struct SessionTZ<TZ: TimeZone> {
    pub start: DateTime<TZ>,
    pub end: DateTime<TZ>,
    pub description: String,
}

pub type Session = SessionTZ<FixedOffset>;

impl Session {
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
}

pub struct SessionIterator {
    lines: Lines<BufReader<File>>,
}

fn is_macro_line(line: impl AsRef<str>, prefix: char) -> bool {
    line.as_ref().starts_with(['%', prefix])
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

impl<TZ: TimeZone> SessionTZ<TZ> {
    pub fn with_timezone<TZ2: TimeZone>(self, tz2: &TZ2) -> SessionTZ<TZ2> {
        SessionTZ {
            start: self.start.with_timezone(tz2),
            end: self.end.with_timezone(tz2),
            description: self.description,
        }
    }

    pub fn naive_local(self) -> NaiveSession {
        NaiveSession {
            start: self.start.naive_local(),
            // use start timezone just in case it differs
            end: self.end.with_timezone(&self.start.timezone()).naive_local(),
            description: self.description,
        }
    }
}

impl NaiveSession {
    pub fn split_at_days(self) -> impl Iterator<Item = Self> {
        let date_start = self.start.date();

        date_start
            .iter_days()
            .zip(date_start.iter_days().skip(1))
            .take_while(move |(d, _tmrw)| d.and_time(NaiveTime::MIN) < self.end)
            .map(move |(d, tmrw)| Self {
                start: self.start.max(d.and_time(NaiveTime::MIN)),
                end: self.end.min(tmrw.and_time(NaiveTime::MIN)),
                description: self.description.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::parser::NaiveSession;

    #[test]
    fn split_at_days() {
        let dt = |year, month, day, h, m, s| {
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(year, month, day).unwrap(),
                NaiveTime::from_hms_opt(h, m, s).unwrap(),
            )
        };
        let sess = |from, to| NaiveSession {
            start: from,
            end: to,
            description: String::new(),
        };

        assert_eq!(
            sess(dt(2000, 1, 1, 0, 0, 0), dt(2000, 1, 2, 0, 0, 1))
                .split_at_days()
                .collect::<Vec<_>>(),
            vec![
                sess(dt(2000, 1, 1, 0, 0, 0), dt(2000, 1, 2, 0, 0, 0)),
                sess(dt(2000, 1, 2, 0, 0, 0), dt(2000, 1, 2, 0, 0, 1))
            ],
        );

        assert_eq!(
            sess(dt(2000, 1, 1, 0, 0, 0), dt(2000, 1, 2, 0, 0, 0))
                .split_at_days()
                .collect::<Vec<_>>(),
            vec![sess(dt(2000, 1, 1, 0, 0, 0), dt(2000, 1, 2, 0, 0, 0)),],
        );

        assert_eq!(
            sess(dt(2000, 1, 1, 12, 0, 0), dt(2000, 1, 3, 12, 0, 0))
                .split_at_days()
                .collect::<Vec<_>>(),
            vec![
                sess(dt(2000, 1, 1, 12, 0, 0), dt(2000, 1, 2, 0, 0, 0)),
                sess(dt(2000, 1, 2, 0, 0, 0), dt(2000, 1, 3, 0, 0, 0)),
                sess(dt(2000, 1, 3, 0, 0, 0), dt(2000, 1, 3, 12, 0, 0)),
            ],
        );
    }
}
