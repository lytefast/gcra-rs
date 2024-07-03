use std::{
    hash::{BuildHasher, Hash},
    time::Instant,
};

use crate::{clock::Clock, GcraError, RateLimit, RateLimiter};

/// A sharded rate limiter implementation using an internal [GcraState] per entry.
/// It is `Send + Sync + Clone` and manages an internal LRU with expiration.
#[derive(Clone)]
pub struct RateLimiterContext<T: Eq + Hash, C, S> {
    pub rate_limit: RateLimit,
    pub rate_limiter: RateLimiter<T, C, S>,
}

impl<Key, C, S> RateLimiterContext<Key, C, S>
where
    Key: Send + Clone + Hash + Eq + 'static,
    C: Clock,
    S: Default + BuildHasher + Clone,
{
    /// Check to see if [key] is rate limited.
    /// # Errors
    /// - [GcraError::DeniedUntil] if the request can succeed after the [Instant] returned.
    /// - [GcraError::DeniedIndefinitely] if the request can never succeed
    #[inline]
    pub async fn check(&self, key: Key, cost: u32) -> Result<Instant, GcraError> {
        self.rate_limiter.check(key, &self.rate_limit, cost).await
    }

    /// Check to see if [key] is rate limited.
    ///
    /// # Errors
    /// - [GcraError::DeniedUntil] if the request can succeed after the [Instant] returned.
    /// - [GcraError::DeniedIndefinitely] if the request can never succeed
    pub async fn check_at(
        &self,
        key: Key,
        cost: u32,
        arrived_at: Instant,
    ) -> Result<Instant, GcraError> {
        self.rate_limiter
            .check_at(key, &self.rate_limit, cost, arrived_at)
            .await
    }
}
