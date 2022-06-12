use std::time::Duration;

pub struct RateLimit {
    pub resource_limit: u32,
    pub period: Duration,

    /// Incremental duration cost of a single resource check
    pub emission_interval: Duration,
}

impl RateLimit {
    pub fn new(resource_limit: u32, period: Duration) -> Self {
        let emission_interval = period / resource_limit;
        Self {
            resource_limit,
            period,
            emission_interval,
        }
    }

    pub fn increment_interval(&self, cost: u32) -> Duration {
        self.emission_interval * cost
    }
}
