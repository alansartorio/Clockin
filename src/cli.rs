use std::ops::Bound;

use chrono::{FixedOffset, Local, NaiveDate};
use clap::{Parser, Subcommand};

const UNBOUNDED_VALUE: &str = "unbounded";

fn parse_bound_naive_date(s: &str) -> Result<Bound<NaiveDate>, String> {
    if s == "unbounded" {
        Ok(Bound::Unbounded)
    } else {
        Ok(Bound::Included(
            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|err| format!("{:#}", err))?,
        ))
    }
}

#[derive(Debug, Parser)]
#[command(name = "Clockin")]
#[command(version = "0.1.0")]
#[command(about = "Time tracking utility", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Link {
        name: String,
    },
    In,
    Summary,
    #[command(alias = "bitacora")]
    Binnacle {
        #[arg(short, long, default_value = UNBOUNDED_VALUE, value_parser = parse_bound_naive_date)]
        from: Bound<NaiveDate>,
        #[arg(short, long, default_value = UNBOUNDED_VALUE, value_parser = parse_bound_naive_date)]
        to: Bound<NaiveDate>,
        #[arg(long, default_value_t = Local::now().fixed_offset().timezone())]
        timezone: FixedOffset,
    },
    Edit,
    Cd,
    Exec {
        command: String,
    },
}
