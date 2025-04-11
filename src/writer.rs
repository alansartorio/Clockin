use std::{fs::File, io::Write, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone};

pub struct Writer {
    file: File,
    pub start: DateTime<Local>,
    closed: bool,
}

fn fmt_datetime<Tz: TimeZone>(time: DateTime<Tz>) -> String {
    time.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

impl Writer {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::options()
            .append(true)
            .open(path)
            .context("opening clockin file")?;

        let start = Local::now();

        let mut instance = Self {
            file,
            start,
            closed: false,
        };

        let start_str = fmt_datetime(start);
        instance
            .write(&format!("%-{}\n", start_str))
            .context("writing start time")?;

        Ok(instance)
    }
    fn write(&mut self, s: &str) -> Result<()> {
        self.file
            .write_all(s.as_bytes())
            .context("writing to .clockin file")
    }
    pub fn write_line(&mut self, line: &str) -> Result<()> {
        self.write(line).context("writing description")?;
        self.write("\n")?;

        Ok(())
    }

    fn write_end(&mut self) -> Result<DateTime<Local>> {
        let end = Local::now();
        self.write(&format!("%+{}\n\n", fmt_datetime(end)))
            .context("writing end time")?;
        self.closed = true;
        Ok(end)
    }

    pub fn end(mut self) -> Result<DateTime<Local>> {
        self.write_end()
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if !self.closed {
            self.write_end().unwrap();
        }
    }
}
