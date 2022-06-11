use chrono::{DateTime, Duration, Utc};

/// Holds the minmum amount of state necessary to implement a GRCA leaky buckets.
/// Refer to: https://blog.ian.stapletoncordas.co/2018/12/understanding-generic-cell-rate-limiting.html
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GcraState {
    /// GCRA's Theoretical Arrival Time timestamp
    pub tat: Option<DateTime<Utc>>,
}

impl GcraState {
    /// Check if we are allowed to proceed. If so updated our internal state and return true.
    /// Explaination of GCRA can be found here:
    /// - https://blog.ian.stapletoncordas.co/2018/12/understanding-generic-cell-rate-limiting.html
    ///
    /// If denied, will return an [Result::Err] where the value is the next allowed timestamp.
    pub fn check_and_modify(
        &mut self,
        rate_limit: &RateLimit,
        cost: u64,
    ) -> Result<(), DateTime<Utc>> {
        let arrived_at = Utc::now();
        self.check_and_modify_internal(rate_limit, arrived_at, cost)
    }

    #[inline]
    fn check_and_modify_internal(
        &mut self,
        rate_limit: &RateLimit,
        arrived_at: DateTime<Utc>,
        cost: u64,
    ) -> Result<(), DateTime<Utc>> {
        let increment_interval = rate_limit.increment_interval(cost);

        let tat = match self.tat {
            Some(tat) => tat,
            None => {
                // First ever request. Allow passage and update self.
                self.tat = Some(arrived_at + increment_interval);
                return Ok(());
            }
        };

        // We had a previous request
        if tat < arrived_at {
            // prev request was really old
            let new_tat = std::cmp::max(tat, arrived_at);
            self.tat = Some(new_tat + increment_interval);
            Ok(())
        } else {
            // prev request was recent and there's a possibility that we've reached the limit
            let delay_variation_tolerance = Duration::milliseconds(rate_limit.period_ms as _);
            let new_tat = tat + increment_interval;

            let next_allowed_at = new_tat - delay_variation_tolerance;
            if next_allowed_at < arrived_at {
                self.tat = Some(new_tat);
                Ok(())
            } else {
                // Denied, must wait until next_allowed_at
                Err(next_allowed_at)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcra_basics() {
        let mut gcra = GcraState::default();
        let rate_limit = RateLimit {
            max_burst: 1,
            period_ms: 1000,
        };

        let first_req_ts = Utc::now();
        assert_eq!(
            Ok(()),
            gcra.check_and_modify(&rate_limit, 1),
            "request #1 should pass"
        );
        let after_first_tat = gcra.tat;
        assert!(
            after_first_tat.is_some(),
            "state should be modified and have a TAT in the future"
        );

        let next_allowed_ts = gcra
            .check_and_modify(&rate_limit, 1)
            .expect_err("request #2 should fail");
        assert!(
            next_allowed_ts >= first_req_ts + Duration::seconds(1),
            "we should only be allowed after the burst period"
        );
        assert_eq!(after_first_tat, gcra.tat, "State should be unchanged.")
    }

    #[test]
    fn gcra_leaky() {
        const INCREMENT_INTERVAL: u64 = 500;

        let mut gcra = GcraState::default();
        let rate_limit = RateLimit {
            max_burst: 3,
            period_ms: 3 * INCREMENT_INTERVAL,
        };

        let arrived_at = Utc::now();
        assert_eq!(
            Ok(()),
            gcra.check_and_modify_internal(&rate_limit, arrived_at, 1),
            "request #1 should pass"
        );
        let single_use_tat = gcra.tat.expect("should have a tat state after use");
        assert_eq!(
            single_use_tat,
            arrived_at + Duration::milliseconds(INCREMENT_INTERVAL as _),
            "new TAT state should have adjusted for leaky bucket"
        );

        assert_eq!(
            Ok(()),
            gcra.check_and_modify(&rate_limit, 2),
            "request #2 should consume all remaining resources and pass"
        );
        assert!(
            matches!(gcra.check_and_modify(&rate_limit, 1), Err(_allowed_at)),
            "request #3 should fail since all resources consumed"
        );

        let current_tat = gcra.tat.expect("should have a tat state after use");
        assert!(current_tat > Utc::now(), "tat in the future");

        assert!(
            matches!(
                // manually force time check that we know will fail
                gcra.check_and_modify_internal(
                    &rate_limit,
                    current_tat
                        - Duration::milliseconds(rate_limit.period_ms as _)
                        - Duration::milliseconds(1),
                    1
                ),
                Err(_allowed_at)
            ),
            "request #4 before leak period should fail. INCREMENT_INTERVAL has not passed yet."
        );

        assert!(
            matches!(
                gcra.check_and_modify_internal(
                    &rate_limit,
                    current_tat - Duration::milliseconds(rate_limit.period_ms as _),
                    1
                ),
                Err(_allowed_at)
            ),
            "request #5 after leak period should pass. INCREMENT_INTERVAL has passed"
        );
    }

    #[test]
    fn gcra_block_expensive_cost_denied() {
        let mut gcra = GcraState::default();
        let rate_limit = RateLimit {
            max_burst: 5,
            period_ms: 1000,
        };

        let first_req_ts = Utc::now();
        assert_eq!(
            Ok(()),
            gcra.check_and_modify(&rate_limit, 1),
            "request #1 should pass"
        );

        let after_first_tat = gcra.tat;
        assert!(
            after_first_tat.is_some(),
            "state should be modified and have a TAT in the future"
        );

        let next_allowed_ts = gcra
            .check_and_modify(&rate_limit, 10)
            .expect_err("request #2 should fail because we couldn't afford the cost");
        assert!(
            next_allowed_ts >= first_req_ts + Duration::seconds(1),
            "we should only be allowed after the burst period"
        );
        assert_eq!(after_first_tat, gcra.tat, "State should be unchanged.")
    }

    #[test]
    fn gcra_refreshed_after_period() {
        let past_time = Utc::now() - Duration::milliseconds(1001);
        let mut gcra = GcraState {
            tat: Some(past_time),
        };
        let rate_limit = RateLimit {
            max_burst: 1,
            period_ms: 1000,
        };
        assert_eq!(
            Ok(()),
            gcra.check_and_modify(&rate_limit, 1),
            "request #1 should pass"
        );

        assert!(
            matches!(gcra.check_and_modify(&rate_limit, 1), Err(_allowed_at)),
            "request #2 should fail"
        );
    }
}
