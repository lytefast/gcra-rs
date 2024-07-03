use std::sync::Arc;

use gcra::{GcraError, RateLimit, RateLimiter};

const CACHE_CAPACITY: usize = 4;
const WORKER_SHARD_COUNT: usize = 2;

#[tokio::main]
async fn main() -> Result<(), GcraError> {
    let rate_limit = RateLimit::per_sec(2);
    let rate_limiter = Arc::new(RateLimiter::with_shards(CACHE_CAPACITY, WORKER_SHARD_COUNT));

    rate_limiter.check("key", &rate_limit, 1).await?;
    rate_limiter.check("key", &rate_limit, 1).await?;

    match rate_limiter.check("key", &rate_limit, 1).await {
        Err(GcraError::DeniedUntil { next_allowed_at }) => {
            print!("Denied: Request next at {:?}", next_allowed_at);
            Ok(())
        }
        unexpected => panic!("Opps something went wrong! {:?}", unexpected),
    }
}
