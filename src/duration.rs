use std::error::Error;
use std::ops::Add;

use chrono::Duration;
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
