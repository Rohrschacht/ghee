use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct DurationParseError;

impl Display for DurationParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error while parsing Duration from String")
    }
}

impl Error for DurationParseError {}
