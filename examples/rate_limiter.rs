use gcra::{GcraError, RateLimit, RateLimiter, RateLimiterError};

const CACHE_CAPACITY: usize = 4;
const WORKER_SHARD_COUNT: u8 = 4;

#[tokio::main]
async fn main() -> Result<(), RateLimiterError> {
    let rate_limit = RateLimit::per_sec(2);
    let mut rl = RateLimiter::new(CACHE_CAPACITY, WORKER_SHARD_COUNT);

    rl.check("key", rate_limit.clone(), 1).await?;
    rl.check("key", rate_limit.clone(), 1).await?;

    match rl.check("key", rate_limit.clone(), 1).await {
        Err(RateLimiterError::GcraError(GcraError::DeniedUntil { next_allowed_at })) => {
            print!("Denied: Request next at {:?}", next_allowed_at);
            Ok(())
        }
        unexpected => panic!("Opps something went wrong! {:?}", unexpected),
    }
}
