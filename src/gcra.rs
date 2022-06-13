use std::time::Instant;

use crate::rate_limit::RateLimit;

/// Holds the minmum amount of state necessary to implement a GRCA leaky buckets.
/// Refer to: [understanding GCRA](https://blog.ian.stapletoncordas.co/2018/12/understanding-generic-cell-rate-limiting.html)
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct GcraState {
    /// GCRA's Theoretical Arrival Time (**TAT**)
    /// An unset value signals a new state
    pub tat: Option<Instant>,
}

impl GcraState {
    /// Check if we are allowed to proceed. If so updated our internal state and return true.
    /// Explaination of GCRA can be found [here](https://blog.ian.stapletoncordas.co/2018/12/understanding-generic-cell-rate-limiting.html)
    ///
    /// # Returns
    /// If denied, will return an [Result::Err] where the value is the next allowed timestamp.
    pub fn check_and_modify(&mut self, rate_limit: &RateLimit, cost: u32) -> Result<(), Instant> {
        let arrived_at = Instant::now();
        self.check_and_modify_internal(rate_limit, arrived_at, cost)
    }

    #[inline]
    fn check_and_modify_internal(
        &mut self,
        rate_limit: &RateLimit,
        arrived_at: Instant,
        cost: u32,
    ) -> Result<(), Instant> {
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
            let delay_variation_tolerance = rate_limit.period;
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
    use std::time::Duration;

    use super::*;

    #[test]
    fn gcra_basics() {
        let mut gcra = GcraState::default();
        let rate_limit = RateLimit::new(1, Duration::from_secs(1));

        let first_req_ts = Instant::now();
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
            next_allowed_ts >= first_req_ts + Duration::from_secs(1),
            "we should only be allowed after the burst period"
        );
        assert_eq!(after_first_tat, gcra.tat, "State should be unchanged.")
    }

    #[test]
    fn gcra_leaky() {
        // const INCREMENT_INTERVAL: u64 = 500;
        const INCREMENT_INTERVAL: Duration = Duration::from_millis(500);

        let mut gcra = GcraState::default();
        let rate_limit = RateLimit::new(10, 10 * INCREMENT_INTERVAL);

        let arrived_at = Instant::now();
        assert_eq!(
            Ok(()),
            gcra.check_and_modify_internal(&rate_limit, arrived_at, 1),
            "request #1 should pass"
        );
        assert_eq!(
            gcra.tat,
            Some(arrived_at + INCREMENT_INTERVAL),
            "new TAT state should have been moved forward according to cost"
        );

        assert_eq!(
            Ok(()),
            gcra.check_and_modify(&rate_limit, 9),
            "request #2 should consume all remaining resources and pass"
        );
        assert!(
            matches!(gcra.check_and_modify(&rate_limit, 1), Err(_allowed_at)),
            "request #3 should fail since all resources consumed"
        );

        let current_tat = gcra.tat.expect("should have a tat state after use");
        assert!(current_tat > Instant::now(), "tat in the future");

        assert!(
            matches!(
                // manually force time check that we know will fail
                gcra.check_and_modify_internal(
                    &rate_limit,
                    current_tat - rate_limit.period - Duration::from_millis(1),
                    1
                ),
                Err(_allowed_at)
            ),
            "request #4 before leak period should fail. INCREMENT_INTERVAL has not passed yet."
        );

        assert!(
            matches!(
                gcra.check_and_modify_internal(&rate_limit, current_tat - rate_limit.period, 1),
                Err(_allowed_at)
            ),
            "request #5 after leak period should pass. INCREMENT_INTERVAL has passed"
        );
    }

    #[test]
    fn gcra_block_expensive_cost_denied() {
        let mut gcra = GcraState::default();
        let rate_limit = RateLimit::new(5, Duration::from_secs(1));

        let first_req_ts = Instant::now();
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
            next_allowed_ts >= first_req_ts + Duration::from_secs(1),
            "we should only be allowed after the burst period"
        );
        assert_eq!(after_first_tat, gcra.tat, "State should be unchanged.")
    }

    #[test]
    fn gcra_refreshed_after_period() {
        let past_time = Instant::now() - Duration::from_millis(1001);
        let mut gcra = GcraState {
            tat: Some(past_time),
        };
        let rate_limit = RateLimit::new(1, Duration::from_secs(1));
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
