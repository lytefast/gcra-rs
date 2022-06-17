use std::{fmt::Display, hash::Hash, time::Instant};
use thingvellir::{Commit, MutableServiceHandle, ShardError};
use thiserror::Error;

use super::{clock::{Clock, InstantClock}, entry::{InMemoryUpstream, RateLimitEntry}};
use crate::{GcraError, RateLimit};

#[derive(Error, Debug)]
pub enum RateLimiterError {
    #[error("Rate Limit error: {0:?}")]
    GcraError(#[from] GcraError),
    #[error("Internal sharding error: {0:?}")]
    ShardError(#[from] ShardError),
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct RateLimitRequest<T> {
    key: T,
}

impl<T> Display for RateLimitRequest<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RateLimitRequest={}", self.key)
    }
}

/// A sharded rate limiter implementation using an internal [GcraState] per entry.
/// It is `Send + Sync + Clone` and manages an internal LRU with expiration.
#[derive(Clone)]
pub struct RateLimiter<T, C = InstantClock> {
    clock: C,
    shard_handle: MutableServiceHandle<RateLimitRequest<T>, RateLimitEntry>,
}

impl<Key> RateLimiter<Key, InstantClock>
where
    Key: Send + Clone + Hash + Eq + Display + 'static,
{
    pub fn new(max_data_capacity: usize, num_shards: u8) -> Self {
        Self {
            clock: InstantClock,
            shard_handle: thingvellir::service_builder(max_data_capacity)
                .num_shards(num_shards)
                .build_mutable(
                    InMemoryUpstream {},
                    thingvellir::DefaultCommitPolicy::Immediate,
                ),
        }
    }
}

impl<Key, C> RateLimiter<Key, C>
where
    Key: Send + Clone + Hash + Eq + Display + 'static,
    C: Clock
{
    /// Check to see if [key] is rate limited.
    /// # Errors
    /// - [GcraError::DeniedUntil] if the request can succeed after the [Instant] returned.
    /// - [GcraError::DeniedIndefinitely] if the request can never succeed
    #[inline]
    pub async fn check(
        &mut self,
        key: Key,
        rate_limit: RateLimit,
        cost: u32,
    ) -> Result<(), RateLimiterError> {
        self.check_at(key, rate_limit, cost, self.clock.now()).await
    }

    /// Check to see if [key] is rate limited.
    ///
    /// # Errors
    /// - [GcraError::DeniedUntil] if the request can succeed after the [Instant] returned.
    /// - [GcraError::DeniedIndefinitely] if the request can never succeed
    pub async fn check_at(
        &mut self,
        key: Key,
        rate_limit: RateLimit,
        cost: u32,
        arrived_at: Instant
    ) -> Result<(), RateLimiterError> {
        let request_key = RateLimitRequest { key };
        let result = self
            .shard_handle
            .execute_mut(request_key, move |entry| {
                let check_result = entry.check_and_modify_at(&rate_limit, arrived_at, cost);

                match check_result {
                    Ok(_) => {
                        entry.update_expiration(&rate_limit);
                        Commit::immediately(check_result)
                    }
                    Err(GcraError::DeniedUntil { .. })
                    | Err(GcraError::DeniedIndefinitely { .. }) => unsafe {
                        Commit::noop(check_result)
                    },
                }
            })
            .await?;
        Ok(result?)
    }
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    async fn rate_limiter_run_until_denied() {
        let rate_limit = RateLimit::new(3, Duration::from_secs(3));
        let mut rl = RateLimiter::new(4, 4);

        for _ in 0..rate_limit.resource_limit {
            assert!(rl.check("key", rate_limit.clone(), 1).await.is_ok());
        }

        match rl.check("key", rate_limit, 1).await {
            Ok(_) => panic!("We should be rate limited"),
            Err(RateLimiterError::GcraError(GcraError::DeniedUntil { next_allowed_at })) => {
                assert!(next_allowed_at > Instant::now())
            }
            Err(_) => panic!("Unexpected error"),
        }
    }

    #[tokio::test]
    async fn rate_limiter_indefinitly_denied() {
        let rate_limit = RateLimit::new(3, Duration::from_secs(3));
        let mut rl = RateLimiter::new(4, 4);

        match rl.check("key", rate_limit.clone(), 9).await {
            Ok(_) => panic!("We should be rate limited"),
            Err(RateLimiterError::GcraError(GcraError::DeniedIndefinitely {
                cost,
                rate_limit: err_rate_limit,
            })) => {
                assert_eq!(cost, 9);
                assert_eq!(err_rate_limit, rate_limit);
            }
            Err(_) => panic!("Unexpected error"),
        }
    }

    #[tokio::test]
    async fn rate_limiter_leaks() {
        let rate_limit = RateLimit::per_sec(2);
        let mut rl = RateLimiter::new(4, 4);

        let now = Instant::now();
        assert!(rl.check_at("key", rate_limit.clone(), 1, now).await.is_ok());
        assert!(rl.check_at("key", rate_limit.clone(), 1, now + Duration::from_millis(250)).await.is_ok(), "delay the 2nd check");
        assert!(rl.check_at("key", rate_limit.clone(), 1, now + Duration::from_millis(251)).await.is_err(), "check we are denied start");
        assert!(rl.check_at("key", rate_limit.clone(), 1, now + Duration::from_millis(499)).await.is_err(), "check we are denied end");
        assert!(rl.check_at("key", rate_limit.clone(), 1, now + Duration::from_millis(501)).await.is_ok(), "1st use should be released")
    }
}
