//! Library which implements the core
//! [GCRA](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm) functionality in rust.
//!
//! # Usage
//!
//! ```rust
//! use gcra::{GcraState, RateLimit};
//!
//! fn check_rate_limit() {
//!   const LIMIT: u32 = 1;
//!   // Create a rate limit that allows `1/1s`
//!   let rate_limit = RateLimit::per_sec(LIMIT);
//!
//!   let mut user_state = GcraState::default();
//!   assert!(user_state.check_and_modify(&rate_limit, 1).is_ok());
//!   assert!(
//!       user_state.check_and_modify(&rate_limit, 1).is_err(),
//!       "We should be over the limit now"
//!   );
//! }
//! ```
//!
//! # Features
//! - `rate-limiter` a LRU + expiring rate limiter. Implements `Send + Sync` so
//!   can be used asynchronously.

mod gcra;
mod rate_limit;
#[cfg(feature = "rate-limiter")]
mod rate_limiter;

pub use crate::gcra::{GcraError, GcraState};
pub use crate::rate_limit::RateLimit;
#[cfg(feature = "rate-limiter")]
pub use crate::rate_limiter::{RateLimiter, RateLimiterError};
