pub struct RateLimit {
    pub max_burst: u32,
    pub period_ms: u64,
}

impl RateLimit {
    pub fn increment_interval(&self, cost: u64) -> Duration {
        let emission_interval_ms = self.period_ms / self.max_burst as u64;
        Duration::milliseconds((emission_interval_ms * cost) as i64)
    }
}
