use std::error::Error;
use std::ops::Add;

use chrono::{Datelike, DateTime, Duration, FixedOffset, Timelike, TimeZone, Weekday};
use chrono::LocalResult::Single;
use regex::Regex;

use crate::error::DurationParseError;

pub fn duration_from_str(s: &str) -> Result<Duration, Box<dyn Error>> {
    let re = Regex::new(r"^(?:(\d+)h)?\s*(?:(\d+)d)?\s*(?:(\d+)w)?\s*(?:(\d+)m)?\s*(?:(\d+)y)?$")
        .unwrap();
    let mut d = Duration::zero();

    if !re.is_match(s) {
        return Err(Box::new(DurationParseError));
    };

    let capture = re.captures(s).unwrap();

    let hours = capture.get(1);
    let days = capture.get(2);
    let weeks = capture.get(3);
    let months = capture.get(4);
    let years = capture.get(5);

    println!("{:?}", hours);
    println!("{:?}", days);
    println!("{:?}", weeks);
    println!("{:?}", months);
    println!("{:?}", years);

    if let Some(h) = hours {
        d = d.add(Duration::hours(h.as_str().parse()?));
    }
    if let Some(days) = days {
        d = d.add(Duration::days(days.as_str().parse()?));
    }
    if let Some(w) = weeks {
        d = d.add(Duration::weeks(w.as_str().parse()?));
    }
    if let Some(m) = months {
        d = d.add(Duration::weeks(4 * m.as_str().parse::<i64>()?));
    }
    if let Some(y) = years {
        d = d.add(Duration::days(365 * y.as_str().parse::<i64>()?));
    }

    Ok(d)
}

pub fn duration_trunc_hour(ts: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    FixedOffset::from_offset(&ts.timezone())
        .ymd(ts.year(), ts.month(), ts.day())
        .and_hms(ts.hour(), 0, 0)
}

pub fn duration_trunc_day(ts: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    FixedOffset::from_offset(&ts.timezone())
        .ymd(ts.year(), ts.month(), ts.day())
        .and_hms(0, 0, 0)
}

pub fn duration_trunc_week(ts: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    let year = ts.year();

    // last week of last year?
    let mon = FixedOffset::from_offset(&ts.timezone())
        .isoywd(year - 1, 52, Weekday::Mon)
        .and_hms(0, 0, 0);
    let sun = FixedOffset::from_offset(&ts.timezone())
        .isoywd(year - 1, 52, Weekday::Sun)
        .and_hms(23, 59, 59);
    if mon <= *ts && *ts <= sun {
        return FixedOffset::from_offset(&ts.timezone())
            .isoywd(year - 1, 52, Weekday::Mon)
            .and_hms(0, 0, 0);
    }

    // last week of last year? (sometimes years have 53 weeks)
    let mon = FixedOffset::from_offset(&ts.timezone()).isoywd_opt(year - 1, 53, Weekday::Mon);
    if let Single(mon) = mon {
        let mon = mon.and_hms(0, 0, 0);
        let sun = FixedOffset::from_offset(&ts.timezone())
            .isoywd(year - 1, 53, Weekday::Sun)
            .and_hms(23, 59, 59);
        if mon <= *ts && *ts <= sun {
            return FixedOffset::from_offset(&ts.timezone())
                .isoywd(year - 1, 53, Weekday::Mon)
                .and_hms(0, 0, 0);
        }
    }

    // any week this year? (rarely years have 53 weeks)
    for i in 1..=53 {
        let mon = FixedOffset::from_offset(&ts.timezone())
            .isoywd(year, i, Weekday::Mon)
            .and_hms(0, 0, 0);
        let sun = FixedOffset::from_offset(&ts.timezone())
            .isoywd(year, i, Weekday::Sun)
            .and_hms(23, 59, 59);
        if mon <= *ts && *ts <= sun {
            return FixedOffset::from_offset(&ts.timezone())
                .isoywd(year, i, Weekday::Mon)
                .and_hms(0, 0, 0);
        }
    }

    // first week of next year?
    let mon = FixedOffset::from_offset(&ts.timezone())
        .isoywd(year + 1, 1, Weekday::Mon)
        .and_hms(0, 0, 0);
    let sun = FixedOffset::from_offset(&ts.timezone())
        .isoywd(year + 1, 1, Weekday::Sun)
        .and_hms(23, 59, 59);
    if mon <= *ts && *ts <= sun {
        return FixedOffset::from_offset(&ts.timezone())
            .isoywd(year + 1, 1, Weekday::Mon)
            .and_hms(0, 0, 0);
    }

    panic!("did not find week");
}

pub fn duration_trunc_month(ts: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    FixedOffset::from_offset(&ts.timezone())
        .ymd(ts.year(), ts.month(), 1)
        .and_hms(0, 0, 0)
}

pub fn duration_trunc_year(ts: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    FixedOffset::from_offset(&ts.timezone())
        .ymd(ts.year(), 1, 1)
        .and_hms(0, 0, 0)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, FixedOffset, Local, TimeZone, Utc, Weekday};

    use crate::duration::{
        duration_trunc_day, duration_trunc_hour, duration_trunc_month, duration_trunc_week,
        duration_trunc_year,
    };

    #[test]
    fn weekdays_as_expected() {
        let mon = Local.isoywd(2022, 5, Weekday::Mon);
        let sun = Local.isoywd(2022, 5, Weekday::Sun);
        let duration = sun - mon;
        assert_eq!(duration, Duration::days(6));

        let mon = Utc.isoywd(2022, 5, Weekday::Mon);
        let sun = Utc.isoywd(2022, 5, Weekday::Sun);
        let duration = sun - mon;
        assert_eq!(duration, Duration::days(6));
    }

    #[test]
    fn week_edge_cases() {
        let cases = Vec::from([
            (
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
                Local.ymd(2021, 12, 27).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 2).and_hms(0, 0, 0),
                Local.ymd(2021, 12, 27).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 3).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 3).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 4).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 3).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 10, 22).and_hms(0, 0, 0),
                Local.ymd(2022, 10, 17).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2020, 12, 29).and_hms(0, 0, 0),
                Local.ymd(2020, 12, 28).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2021, 1, 2).and_hms(0, 0, 0),
                Local.ymd(2020, 12, 28).and_hms(0, 0, 0),
            ),
        ]);

        for (date, week_trunced) in cases {
            let fo_date = date.with_timezone(date.offset());
            let calculated = duration_trunc_week(&fo_date);
            assert_eq!(calculated, week_trunced);
        }
    }

    #[test]
    fn years() {
        let cases = Vec::from([
            (
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 2).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 30).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 31).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
        ]);

        for (date, year_trunced) in cases {
            let fo_date = date.with_timezone(date.offset());
            let calculated = duration_trunc_year(&fo_date);
            assert_eq!(calculated, year_trunced);
        }
    }

    #[test]
    fn days() {
        let cases = Vec::from([
            (
                Local.ymd(2022, 1, 1).and_hms(1, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 2).and_hms(10, 0, 0),
                Local.ymd(2022, 1, 2).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 30).and_hms(23, 59, 59),
                Local.ymd(2022, 12, 30).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 31).and_hms(15, 38, 17),
                Local.ymd(2022, 12, 31).and_hms(0, 0, 0),
            ),
        ]);

        for (date, day_trunced) in cases {
            let fo_date = date.with_timezone(date.offset());
            let calculated = duration_trunc_day(&fo_date);
            assert_eq!(calculated, day_trunced);
        }
    }

    #[test]
    fn months() {
        let cases = Vec::from([
            (
                Local.ymd(2022, 1, 1).and_hms(1, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 2).and_hms(10, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 30).and_hms(23, 59, 59),
                Local.ymd(2022, 12, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 31).and_hms(15, 38, 17),
                Local.ymd(2022, 12, 1).and_hms(0, 0, 0),
            ),
        ]);

        for (date, month_trunced) in cases {
            let fo_date = date.with_timezone(date.offset());
            let calculated = duration_trunc_month(&fo_date);
            assert_eq!(calculated, month_trunced);
        }
    }

    #[test]
    fn hours() {
        let cases = Vec::from([
            (
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
                Local.ymd(2022, 1, 1).and_hms(0, 0, 0),
            ),
            (
                Local.ymd(2022, 1, 2).and_hms(10, 0, 0),
                Local.ymd(2022, 1, 2).and_hms(10, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 30).and_hms(23, 59, 59),
                Local.ymd(2022, 12, 30).and_hms(23, 0, 0),
            ),
            (
                Local.ymd(2022, 12, 31).and_hms(15, 38, 17),
                Local.ymd(2022, 12, 31).and_hms(15, 0, 0),
            ),
        ]);

        for (date, hour_trunced) in cases {
            let fo_date = date.with_timezone(date.offset());
            let calculated = duration_trunc_hour(&fo_date);
            assert_eq!(calculated, hour_trunced);
        }
    }
}
