use std::error::Error;

use regex::Regex;

use crate::error::DurationParseError;

pub struct Retention {
    pub h: usize,
    pub d: usize,
    pub w: usize,
    pub m: usize,
    pub y: usize,
}

impl Retention {
    pub fn zero() -> Self {
        Retention {
            h: 0,
            d: 0,
            w: 0,
            m: 0,
            y: 0,
        }
    }

    pub fn from_str_option(o: &Option<String>) -> Result<Self, Box<dyn Error>> {
        match o {
            None => Ok(Self::zero()),
            Some(s) => Self::from_str(&s[..]),
        }
    }

    pub fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        let re = Regex::new(r"^(?:(\d+)h)?\s*(?:(\d+)d)?\s*(?:(\d+)w)?\s*(?:(\d+)m)?\s*(?:(\d+)y)?$")?;

        if !re.is_match(s) {
            return Err(Box::new(DurationParseError));
        };

        let capture = re.captures(s).ok_or(Box::new(DurationParseError))?;

        let hours = capture.get(1);
        let days = capture.get(2);
        let weeks = capture.get(3);
        let months = capture.get(4);
        let years = capture.get(5);

        let mut r = Retention::zero();

        if let Some(h) = hours {
            r.h = h.as_str().parse()?
        }
        if let Some(days) = days {
            r.d = days.as_str().parse()?
        }
        if let Some(w) = weeks {
            r.w = w.as_str().parse()?
        }
        if let Some(m) = months {
            r.m = m.as_str().parse()?
        }
        if let Some(y) = years {
            r.y = y.as_str().parse()?
        }

        Ok(r)
    }
}
