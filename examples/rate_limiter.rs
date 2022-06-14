use gcra::{GcraError, RateLimit, RateLimiter, RateLimiterError};

#[tokio::main]
async fn main() -> Result<(), RateLimiterError> {
    let rate_limit = RateLimit::per_sec(1);
    let mut rl = RateLimiter::new(4, 4);

    rl.check("key", rate_limit.clone(), 1).await?;
    match rl.check("key", rate_limit.clone(), 1).await {
        Err(RateLimiterError::GcraError(GcraError::DeniedUntil { next_allowed_at })) => {
            print!("Request next at {:?}", next_allowed_at);
            Ok(())
        }
        unexpected => panic!("Opps something went wrong! {:?}", unexpected),
    }
}
