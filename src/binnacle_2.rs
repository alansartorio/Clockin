use std::time::Duration;

use chrono::{Datelike, FixedOffset, NaiveDate, TimeZone};
use itertools::Itertools;

use crate::{
    binnacle_body_parser::{self, SessionWithBody},
    format_util::{fmt_duration_uncertain, fmt_month, fmt_weekday},
    parser::{NaiveSessionIteratorExt, SessionIteratorExt, SessionTZ},
    summary::{MonthId, NaiveDateExt},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Task {
    subject: String,
}

#[derive(Debug, Clone)]
pub struct SubProjectDayInfo {
    total_time: Duration,
    tasks: Vec<Task>,
}

#[derive(Debug)]
pub struct SubProjectDay {
    sub_project_name: String,
    info: SubProjectDayInfo,
}

#[derive(Debug)]
pub struct Day {
    date: NaiveDate,
    sub_projects: Vec<SubProjectDay>,
}

#[derive(Debug)]
pub struct Month {
    id: MonthId,
    total_time: Duration,
    days: Vec<Day>,
}

#[derive(Debug)]
pub struct BinnacleData {
    months: Vec<Month>,
}

pub fn process(
    sessions: impl Iterator<Item = SessionTZ<FixedOffset>>,
    timezone: &impl TimeZone,
) -> BinnacleData {
    BinnacleData {
        months: sessions
            .with_timezone(timezone)
            .naive_local()
            .cut_at_days()
            .map(|s| SessionWithBody {
                body: binnacle_body_parser::parse(&s.description)
                    .unwrap()
                    .to_owned(),
                session: s,
            })
            .chunk_by(|s| s.session.start.date().month_id())
            .into_iter()
            .map(|(month_id, sessions)| {
                let month_sessions = sessions.collect_vec();
                Month {
                    id: month_id,
                    total_time: month_sessions
                        .iter()
                        .map(|s| s.session.duration().to_std().unwrap())
                        .sum(),
                    days: month_sessions
                        .into_iter()
                        .chunk_by(|s| s.session.start.date())
                        .into_iter()
                        .map(|(day, chunk)| Day {
                            date: day,
                            sub_projects: chunk
                                .into_grouping_map_by(|s| s.body.sub_project.clone())
                                .fold(
                                    SubProjectDayInfo {
                                        total_time: Duration::ZERO,
                                        tasks: vec![],
                                    },
                                    |mut acc, sub_project, task| {
                                        acc.total_time += task.session.duration().to_std().unwrap();
                                        acc.tasks.push(Task {
                                            subject: task.body.subject.to_owned(),
                                        });

                                        acc
                                    },
                                )
                                .into_iter()
                                .map(|(sub_project, info)| SubProjectDay {
                                    sub_project_name: sub_project
                                        .unwrap_or("sin categoría".to_owned()),
                                    info,
                                })
                                .sorted_by_key(|sub_project_day| {
                                    sub_project_day.sub_project_name.clone()
                                })
                                .collect_vec(),
                        })
                        .collect_vec(),
                }
            })
            .collect_vec(),
    }
}

pub fn format(binnacle_data: BinnacleData, current_date: NaiveDate) {
    for month in binnacle_data.months {
        println!(
            "## {} ({})\n",
            fmt_month(month.id),
            fmt_duration_uncertain(&month.total_time, current_date > month.id.last_day())
        );

        for day in month.days {
            println!("{}\n", day.date.format("%d/%m/%Y"));
            for sub_project in day.sub_projects {
                println!("({}: {} hs)\n", sub_project.sub_project_name, fmt_duration_uncertain(&sub_project.info.total_time, current_date > day.date));
                for task in sub_project.info.tasks.iter().unique() {
                    println!("\t- {}\n", task.subject);
                }
            }
            println!("\n");
        }
    }
}
