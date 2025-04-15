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
#[command(version)]
#[command(about = "Time tracking utility", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "create a project and link the current directory to it")]
    Link {
        name: String,
    },
    #[command(about = "start a time tracking session")]
    In,
    WeekSummary,
    #[command(alias = "bitacora", about = "print a report of time spent on the project broken down by month and by day")]
    Summary {
        #[arg(short, long, default_value = UNBOUNDED_VALUE, value_parser = parse_bound_naive_date)]
        from: Bound<NaiveDate>,
        #[arg(short, long, default_value = UNBOUNDED_VALUE, value_parser = parse_bound_naive_date)]
        to: Bound<NaiveDate>,
        #[arg(long, default_value_t = Local::now().fixed_offset().timezone())]
        timezone: FixedOffset,
    },
    #[command(about = "open the project times file in the editor")]
    Edit,
    #[command(about = "open a subshell inside the clockin data directory, respects SHELL environment variable")]
    Cd,
    #[command(about = "execute a command inside the clockin data directory, useful for syncing/git commands, respects EDITOR environment variable")]
    Exec {
        command: String,
    },
}
