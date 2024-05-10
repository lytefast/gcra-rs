use std::time::Instant;

use chrono::{DateTime, Duration, Utc};
use gcra::{GcraError, RateLimit, RateLimitGuard};

fn check_rate_limit(rate_limit_guard: &mut RateLimitGuard) -> bool {
    const COST: u32 = 1;
    match rate_limit_guard.check_and_modify(COST) {
        Ok(_) => {
            let remaining_resources = rate_limit_guard.remaining_resources();
            println!("allowed. Remaining usages: {}", remaining_resources);
            true
        }
        Err(GcraError::DeniedUntil { next_allowed_at }) => {
            println!("denied. Try again at {:?}", to_date_time(next_allowed_at));
            false
        }

        Err(error) => {
            println!("denied: {:?}", error);
            false
        }
    }
}

fn to_date_time(instant: Instant) -> DateTime<Utc> {
    let diff = instant - Instant::now();
    Utc::now() + Duration::from_std(diff).unwrap()
}

fn main() {
    const LIMIT: u32 = 3;
    // Create a rate limit that allows `3/1s`
    let rate_limit = RateLimit::per_sec(LIMIT);
    let mut rate_limit_guard = RateLimitGuard::new_state(rate_limit);

    for i in 0..LIMIT {
        assert!(
            check_rate_limit(&mut rate_limit_guard),
            "Attempt #{} should be allowed",
            i + 1
        );
    }
    assert!(
        !check_rate_limit(&mut rate_limit_guard),
        "We should be over the limit now"
    );

    rate_limit_guard
        .revert(1)
        .expect("Revert should have worked");
    println!(
        "We reverted once, remaining usages: {}",
        rate_limit_guard.remaining_resources(),
    );

    assert!(
        check_rate_limit(&mut rate_limit_guard),
        "Revert should allow additional attempt"
    );
}
