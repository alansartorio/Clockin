use std::{cmp::Ordering, collections::BTreeMap, ops::RangeInclusive, time::Duration};

use chrono::{Datelike, NaiveDate, NaiveWeek};

use crate::parser::Session;

#[derive(Debug, Clone, Copy, Eq)]
pub struct FixedWeek(NaiveWeek);

impl FixedWeek {
    pub fn first_day(&self) -> NaiveDate {
        self.0.first_day()
    }
}

impl PartialEq for FixedWeek {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Ord for FixedWeek {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.first_day().cmp(&other.0.first_day())
    }
}

impl PartialOrd for FixedWeek {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct MonthId {
    pub year: u32,
    pub month: u8,
}

pub trait NaiveDateExt {
    fn month_id(&self) -> MonthId;
    fn real_week(&self) -> FixedWeek;
}

impl NaiveDateExt for NaiveDate {
    fn month_id(&self) -> MonthId {
        let year = self.year_ce().1;
        let month = self.month() as u8;
        MonthId { year, month }
    }

    fn real_week(&self) -> FixedWeek {
        FixedWeek(self.week(chrono::Weekday::Mon))
    }
}

pub struct Day {
    pub duration: Duration,
    pub descriptions: Vec<String>,
}

pub struct Summary {
    pub days: BTreeMap<NaiveDate, Day>,
}

impl Summary {
    pub fn duration(&self, range: RangeInclusive<NaiveDate>) -> Duration {
        self.days
            .range(range)
            .map(|(_date, day)| day.duration)
            .sum()
    }
    pub fn week_duration(&self, week: FixedWeek) -> Duration {
        self.duration(week.0.first_day()..=week.0.last_day())
    }
}

impl Summary {
    pub fn summarize(sessions: impl Iterator<Item = Session>) -> Self {
        let mut summary = Summary {
            days: Default::default(),
        };

        for session in sessions {
            let date = session.start.date_naive();
            let duration = session.duration().to_std().unwrap();
            if summary
                .days
                .last_entry()
                .is_none_or(|last_date| last_date.key() != &date)
            {
                summary.days.insert(
                    date,
                    Day {
                        duration: Duration::ZERO,
                        descriptions: vec![],
                    },
                );
            }

            let mut last_entry = summary.days.last_entry().unwrap();
            let last_entry = last_entry.get_mut();
            last_entry.duration += duration;
            last_entry.descriptions.push(session.description);
        }
        summary
    }
}
