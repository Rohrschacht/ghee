use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Sub;
use std::rc::Rc;

use chrono::{DateTime, Duration, FixedOffset, Local};
use log::trace;

use crate::duration::{
    duration_trunc_day, duration_trunc_hour, duration_trunc_month, duration_trunc_week,
    duration_trunc_year,
};
use crate::intent::IntentType;
use crate::retention::Retention;
use crate::Intent;

#[derive(Debug)]
pub struct TimeBins<'a> {
    pub h: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent<'a>>>>,
    pub rh: Vec<DateTime<FixedOffset>>,
    pub d: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent<'a>>>>,
    pub rd: Vec<DateTime<FixedOffset>>,
    pub w: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent<'a>>>>,
    pub rw: Vec<DateTime<FixedOffset>>,
    pub m: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent<'a>>>>,
    pub rm: Vec<DateTime<FixedOffset>>,
    pub y: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent<'a>>>>,
    pub ry: Vec<DateTime<FixedOffset>>,
}

impl<'a> TimeBins<'a> {
    pub fn new(retention: &Retention) -> Self {
        let mut h: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent>>> = HashMap::new();
        let mut rh: Vec<DateTime<FixedOffset>> = Vec::new();
        let mut d: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent>>> = HashMap::new();
        let mut rd: Vec<DateTime<FixedOffset>> = Vec::new();
        let mut w: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent>>> = HashMap::new();
        let mut rw: Vec<DateTime<FixedOffset>> = Vec::new();
        let mut m: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent>>> = HashMap::new();
        let mut rm: Vec<DateTime<FixedOffset>> = Vec::new();
        let mut y: HashMap<DateTime<FixedOffset>, Rc<RefCell<Intent>>> = HashMap::new();
        let mut ry: Vec<DateTime<FixedOffset>> = Vec::new();

        let now: DateTime<FixedOffset> = Local::now().into();

        let this_hour = duration_trunc_hour(&now);
        for i in 0..=retention.h {
            let bin_hour = this_hour.sub(Duration::hours(i as i64));
            rh.push(bin_hour);
        }

        let this_day = duration_trunc_day(&now);
        for i in 0..=retention.d {
            let bin_day = this_day.sub(Duration::days(i as i64));
            rd.push(bin_day);
        }

        let this_week = duration_trunc_week(&now);
        for i in 0..=retention.w {
            let bin_week = this_week.sub(Duration::weeks(i as i64));
            rw.push(bin_week);
        }

        let this_month = duration_trunc_month(&now);
        for i in 0..=retention.m {
            let bin_month = this_month.sub(Duration::weeks(4 * i as i64));
            rm.push(bin_month);
        }

        let this_year = duration_trunc_year(&now);
        for i in 0..=retention.y {
            let bin_year = this_year.sub(Duration::days(365 * i as i64));
            ry.push(bin_year);
        }

        Self {
            h,
            rh,
            d,
            rd,
            w,
            rw,
            m,
            rm,
            y,
            ry,
        }
    }

    pub fn store(
        &mut self,
        intent_timestamp: &DateTime<FixedOffset>,
        intent: Rc<RefCell<Intent<'a>>>,
    ) {
        let ts_hourly = duration_trunc_hour(intent_timestamp);
        let ts_daily = duration_trunc_day(intent_timestamp);
        let ts_weekly = duration_trunc_week(intent_timestamp);
        let ts_monthly = duration_trunc_month(intent_timestamp);
        let ts_yearly = duration_trunc_year(intent_timestamp);

        trace!("from ts: {:?} ts_hourly: {:?}", intent_timestamp, ts_hourly);
        trace!("from ts: {:?} ts_daily: {:?}", intent_timestamp, ts_daily);
        trace!("from ts: {:?} ts_weekly: {:?}", intent_timestamp, ts_weekly);
        trace!(
            "from ts: {:?} ts_monthly: {:?}",
            intent_timestamp,
            ts_monthly
        );
        trace!("from ts: {:?} ts_yearly: {:?}", intent_timestamp, ts_yearly);

        if self.rh.contains(&ts_hourly) {
            self.h.insert(ts_hourly, intent);
        } else if self.rd.contains(&ts_daily) {
            self.d.insert(ts_daily, intent);
        } else if self.rw.contains(&ts_weekly) {
            self.w.insert(ts_weekly, intent);
        } else if self.rm.contains(&ts_monthly) {
            self.m.insert(ts_monthly, intent);
        } else if self.ry.contains(&ts_yearly) {
            self.y.insert(ts_yearly, intent);
        }
    }

    pub fn set_keep(&self) {
        for int in self.h.values() {
            (**int).borrow_mut().intent = IntentType::Keep;
        }
        for int in self.d.values() {
            (**int).borrow_mut().intent = IntentType::Keep;
        }
        for int in self.w.values() {
            (**int).borrow_mut().intent = IntentType::Keep;
        }
        for int in self.m.values() {
            (**int).borrow_mut().intent = IntentType::Keep;
        }
        for int in self.y.values() {
            (**int).borrow_mut().intent = IntentType::Keep;
        }
    }
}
