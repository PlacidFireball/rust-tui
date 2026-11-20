use std::time::Duration;

use tokio::time::{Interval, MissedTickBehavior, interval};

pub(crate) fn interval_from_millis(millis: Option<u64>) -> Option<Interval> {
    millis.map(|millis| {
        let mut refresh = interval(Duration::from_millis(millis));
        refresh.set_missed_tick_behavior(MissedTickBehavior::Delay);
        refresh
    })
}
