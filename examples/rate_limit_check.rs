use gcra::{GcraState, RateLimit};
use std::time::Duration;

fn check_rate_limit(rate_limit: &RateLimit, gcra_state: &mut GcraState) -> bool {
    const COST: u32 = 1;
    match gcra_state.check_and_modify(rate_limit, COST) {
        Ok(_) => {
            println!("allowed");
            true
        }
        Err(next_allowed_at) => {
            println!("denied. Try again at {:?}", next_allowed_at);
            false
        }
    }
}

fn main() {
    const LIMIT: u32 = 5;
    const PERIOD: Duration = Duration::from_secs(1);
    // Create a rate limit that allows `5/1s`
    let rate_limit = RateLimit::new(LIMIT, PERIOD);

    let mut user_state = GcraState::default();
    for _ in 0..LIMIT {
        assert!(check_rate_limit(&rate_limit, &mut user_state));
    }
    assert!(
        !check_rate_limit(&rate_limit, &mut user_state),
        "We should be over the limit now"
    );
}
