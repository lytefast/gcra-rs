use std::time::Instant;

/// Abstraction for getting time.
pub trait Clock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

pub struct InstantClock;
impl Clock for InstantClock {}
