use std::time::Duration;

use chrono::{NaiveTime, Timelike, Weekday};

use crate::summary::MonthId;



pub fn fmt_duration(duration: &Duration) -> String {
    let duration = duration.as_secs();
    let hours = duration / (60 * 60);
    let total_minutes = duration / 60;
    let minutes = total_minutes - hours * 60;
    let seconds = duration - total_minutes * 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn fmt_duration_uncertain(duration: &Duration, completed: bool) -> String {
    let mut out = fmt_duration(duration);
    if !completed {
        out.push_str(" (incompleto)");
    }

    out
}

pub fn fmt_month(month: MonthId) -> String {
    let month_name = [
        "Enero",
        "Febrero",
        "Marzo",
        "Abril",
        "Mayo",
        "Junio",
        "Julio",
        "Agosto",
        "Septiembre",
        "Octubre",
        "Noviembre",
        "Diciembre",
    ][month.month() as usize];
    format!("{} {}", month_name, month.year())
}

pub fn fmt_weekday(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Lunes",
        Weekday::Tue => "Martes",
        Weekday::Wed => "Miércoles",
        Weekday::Thu => "Jueves",
        Weekday::Fri => "Viernes",
        Weekday::Sat => "Sabado",
        Weekday::Sun => "Domingo",
    }
}

pub fn fmt_hours_mins(t: NaiveTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}
