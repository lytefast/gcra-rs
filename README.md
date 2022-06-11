# GCRA: A basic implementation

This library implements the core GCRA functionality in rust.

## Usage

```rust
use ::gcra::{GcraState, RateLimit};

fn check_rate_limit() {
  const LIMIT: u32 = 1;
  const PERIOD: Duration = Duration::from_secs(1);
  // Create a rate limit that allows `5/1s`
  let rate_limit = RateLimit::new(LIMIT, PERIOD);

  let mut user_state = GcraState::default();
  assert!(user_state.check_and_modify(&rate_limit, 1).is_ok());
  assert!(
      !assert!(user_state.check_and_modify(&rate_limit, 1).is_err(),
      "We should be over the limit now"
  );
}
```
