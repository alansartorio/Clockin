use std::{fs::File, io::Write, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone};

fn fmt_datetime<Tz: TimeZone>(time: DateTime<Tz>) -> String {
    time.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

pub fn write_date(path: impl AsRef<Path>, extra_return: bool) -> Result<()> {
    let mut file = File::options()
        .append(true)
        .open(path)
        .context("opening clockin file")?;

    let start = Local::now();

    let start_str = fmt_datetime(start);
    file.write_all(format!("%-{}\n", start_str).as_bytes())
        .context("writing start time")?;

    if extra_return {
        file.write_all("\n".as_bytes())
            .context("writing start time")?;
    }
    Ok(())
}
