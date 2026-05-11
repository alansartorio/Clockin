use std::{fs::File, io::Write, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone};

fn fmt_datetime<Tz: TimeZone>(time: DateTime<Tz>) -> String {
    time.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

pub fn write_date(path: impl AsRef<Path>, extra_return: bool, prefix: char, time: DateTime<impl TimeZone>) -> Result<()> {
    let mut file = File::options()
        .append(true)
        .open(path)
        .context("opening clockin file")?;

    let time_str = fmt_datetime(time);
    file.write_all(format!("%{prefix}{time_str}\n").as_bytes())
        .context("writing time")?;

    if extra_return {
        file.write_all("\n".as_bytes())
            .context("writing time")?;
    }
    Ok(())
}
