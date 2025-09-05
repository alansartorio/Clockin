use std::{cmp::Ordering, collections::{BTreeMap, HashSet}, ops::RangeBounds, time::Duration};

use chrono::{Datelike, Days, Months, NaiveDate, NaiveWeek, TimeZone};

use crate::parser::{NaiveSessionIteratorExt, Session, SessionIteratorExt};

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
    year: u32,
    month: u8,
}

impl MonthId {
    pub fn new(year: u32, month: u8) -> Self {
        assert!(month < 12);
        Self { year, month }
    }

    pub fn year(&self) -> u32 {
        self.year
    }
    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn first_day(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year as i32, self.month as u32 + 1, 1).unwrap()
    }
    pub fn last_day(&self) -> NaiveDate {
        self.first_day()
            .checked_add_months(Months::new(1))
            .unwrap()
            .checked_sub_days(Days::new(1))
            .unwrap()
    }
}

pub trait NaiveDateExt {
    fn month_id(&self) -> MonthId;
    fn real_week(&self) -> FixedWeek;
}

impl NaiveDateExt for NaiveDate {
    fn month_id(&self) -> MonthId {
        let year = self.year_ce().1;
        let month = self.month0() as u8;
        MonthId::new(year, month)
    }

    fn real_week(&self) -> FixedWeek {
        FixedWeek(self.week(chrono::Weekday::Mon))
    }
}

pub struct Day {
    pub duration: Duration,
    pub descriptions: HashSet<String>,
}

pub struct Summary {
    pub days: BTreeMap<NaiveDate, Day>,
}

impl Summary {
    pub fn duration(&self, range: impl RangeBounds<NaiveDate>) -> Duration {
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
    pub fn summarize<Tz: TimeZone>(sessions: impl Iterator<Item = Session>, timezone: &Tz) -> Self {
        let mut summary = Summary {
            days: Default::default(),
        };

        for session in sessions.with_timezone(timezone).naive_local().cut_at_days() {
            let date = session.start.date();
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
                        descriptions: HashSet::new(),
                    },
                );
            }

            let mut last_entry = summary.days.last_entry().unwrap();
            let last_entry = last_entry.get_mut();
            last_entry.duration += duration;
            if !session.description.is_empty() {
                last_entry.descriptions.insert(session.description);
            }
        }
        summary
    }
}
