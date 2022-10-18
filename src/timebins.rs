use chrono::{DateTime, Duration, DurationRound, FixedOffset, TimeZone, Utc};

pub struct TimeBins {
    pub h: DateTime<FixedOffset>,
    pub d: DateTime<FixedOffset>,
    pub w: DateTime<FixedOffset>,
    pub m: DateTime<FixedOffset>,
    pub y: DateTime<FixedOffset>,
}

impl TimeBins {
    pub fn new(ts: &DateTime<FixedOffset>) -> Self {
        Self {
            h: ts.duration_trunc(Duration::hours(1)).unwrap(),
            d: ts.duration_trunc(Duration::days(1)).unwrap(),
            w: ts.duration_trunc(Duration::weeks(1)).unwrap(),
            m: ts.duration_trunc(Duration::weeks(4)).unwrap(),
            y: ts.duration_trunc(Duration::days(365)).unwrap(),
        }
    }

    pub fn oldest() -> Self {
        Self {
            h: Utc.ymd(0, 1, 1).and_hms(0, 0, 0).into(),
            d: Utc.ymd(0, 1, 1).and_hms(0, 0, 0).into(),
            w: Utc.ymd(0, 1, 1).and_hms(0, 0, 0).into(),
            m: Utc.ymd(0, 1, 1).and_hms(0, 0, 0).into(),
            y: Utc.ymd(0, 1, 1).and_hms(0, 0, 0).into(),
        }
    }
}
