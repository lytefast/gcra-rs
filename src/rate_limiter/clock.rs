use std::time::Instant;

/// Abstraction for getting time.
pub trait Clock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantClock;
impl Clock for InstantClock {}
