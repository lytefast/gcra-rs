mod entry;
mod rate_limiter;
mod rate_limiter_context;

pub use entry::*;
pub use rate_limiter::*;
pub use rate_limiter_context::*;

use crate::RateLimit;
use std::hash::Hash;

impl<Key, C, S> RateLimiter<Key, C, S>
where
    Key: Eq + Hash,
{
    pub fn into_rate_limit_context(self, rate_limit: RateLimit) -> RateLimiterContext<Key, C, S> {
        RateLimiterContext {
            rate_limit,
            rate_limiter: self,
        }
    }
}
