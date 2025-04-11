use std::{cmp::Ordering, collections::BTreeMap, time::Duration};

use chrono::{NaiveDate, NaiveWeek};

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

pub struct Day {
    pub duration: Duration,
    pub descriptions: Vec<String>,
}

pub struct Week {
    pub days: BTreeMap<NaiveDate, Day>,
}

impl Week {
    pub fn duration(&self) -> Duration {
        self.days.values().map(|d| d.duration).sum()
    }
}

pub struct Summary {
    pub weeks: BTreeMap<FixedWeek, Week>,
}

impl Summary {
    pub fn summarize(sessions: impl Iterator<Item = Session>) -> Self {
        let mut summary = Summary {
            weeks: Default::default(),
        };

        for session in sessions {
            let date = session.start.date_naive();
            let week = FixedWeek(date.week(chrono::Weekday::Mon));
            let duration = session.duration().to_std().unwrap();
            let last_week = summary.weeks.last_entry();

            let create_week = last_week
                .as_ref()
                .is_none_or(|last_week| last_week.key() != &week);
            let last_day = (!create_week)
                .then(move || {
                    last_week
                        .unwrap()
                        .get()
                        .days
                        .last_key_value()
                        .map(|(date, _duration)| *date)
                })
                .flatten();

            let create_day = last_day.is_none_or(|last_day| last_day != date);

            if create_week {
                summary.weeks.insert(
                    week,
                    Week {
                        days: Default::default(),
                    },
                );
            }

            let mut last_week = summary.weeks.last_entry().unwrap();
            if create_day {
                last_week.get_mut().days.insert(
                    date,
                    Day {
                        descriptions: vec![],
                        duration: Duration::ZERO,
                    },
                );
            }

            let mut last_day = last_week.get_mut().days.last_entry().unwrap();

            last_day.get_mut().duration += duration;
            last_day.get_mut().descriptions.push(session.description);
        }
        summary
    }
}
