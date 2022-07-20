use std::{collections::HashMap, time::Duration, hash::Hash, thread, sync::{RwLock, Arc}};

use gcra::{GcraError, RateLimit, RateLimiter, RateLimiterError, RateLimitEntry};
use thingvellir::{LoadFromUpstream, CommitToUpstream};


const CACHE_CAPACITY: usize = 4;
const WORKER_SHARD_COUNT: u8 = 4;

const IO_DURATION: Duration = Duration::from_millis(50);

#[tokio::main]
async fn main() -> Result<(), RateLimiterError> {
    let rate_limit = RateLimit::per_sec(2);

    // Use a persistence backed rate limiter
    let upstream_factory = FakePersistenceUpstream {
        data: Arc::new(RwLock::new(HashMap::new()))
    };
    let mut rl = RateLimiter::with_handle(
        thingvellir::service_builder(CACHE_CAPACITY)
            .num_shards(WORKER_SHARD_COUNT)
            .build_mutable(upstream_factory, thingvellir::DefaultCommitPolicy::Immediate)
    );

    rl.check("key", rate_limit.clone(), 1).await?;
    rl.check("key", rate_limit.clone(), 1).await?;

    match rl.check("key", rate_limit.clone(), 1).await {
        Err(RateLimiterError::GcraError(GcraError::DeniedUntil { next_allowed_at })) => {
            println!("Denied: Request next at {:?}", next_allowed_at);

            let duration = Duration::from_millis(10);
            println!("Sleep for {:?} to allow commits", duration);
            tokio::time::sleep(duration).await;
        }
        unexpected => panic!("Opps something went wrong! {:?}", unexpected),
    };

    Ok(())
}

#[derive(Clone)]
struct FakePersistenceUpstream<K, D>{data: Arc<RwLock<HashMap<K, D>>>}


impl<Key> LoadFromUpstream<Key, RateLimitEntry> for FakePersistenceUpstream<Key, RateLimitEntry>
where
    Key: 'static + Send + Sync + Hash + Eq + Clone,
{
    fn load(&mut self, request: thingvellir::DataLoadRequest<Key, RateLimitEntry>) {
        let key = request.key().clone();
        let data = self.data.clone();

        request.spawn_default(async move {
            println!("LOAD. Sleeping for {:?}", IO_DURATION);
            tokio::time::sleep(IO_DURATION).await;

            match data.read().expect("RwLock poisoned").get(&key) {
                Some(value) => {
                    println!("LOADED {:?}", value);
                    Ok(value.clone())},
                None => {
                    println!("LOADED NOT_FOUND");
                    Err(thingvellir::UpstreamError::KeyNotFound)
                },
            }
        });
    }
}

impl<Key> CommitToUpstream<Key, RateLimitEntry> for FakePersistenceUpstream<Key, RateLimitEntry>
where 
    Key: 'static + Send + Sync + Hash + Eq + Clone,
{
    fn commit<'a>(&mut self, request: thingvellir::DataCommitRequest<'a, Key, RateLimitEntry>) {
        let key = request.key().clone();
        let data = request.data().clone();
        
        println!("COMMIT. Sleeping for {:?}", IO_DURATION);
        thread::sleep(IO_DURATION);

        self.data.write().expect("RwLock poisoned").insert(key, data);
        request.resolve()
    }
}
