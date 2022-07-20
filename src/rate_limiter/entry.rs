use std::{
    ops::{Deref, DerefMut},
    time::Instant,
};

use thingvellir::{
    CommitToUpstream, DataCommitRequest, DataLoadRequest, LoadFromUpstream, ServiceData,
};

use crate::{GcraState, RateLimit};

#[derive(Default, Debug, Clone)]
pub struct RateLimitEntry {
    pub gcra_state: GcraState,
    expires_at: Option<tokio::time::Instant>,
}

impl Deref for RateLimitEntry {
    type Target = GcraState;

    fn deref(&self) -> &Self::Target {
        &self.gcra_state
    }
}

impl DerefMut for RateLimitEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.gcra_state
    }
}

impl ServiceData for RateLimitEntry {
    fn should_persist(&self) -> bool {
        true
    }

    fn get_expires_at(&self) -> Option<&tokio::time::Instant> {
        self.expires_at.as_ref()
    }
}

impl RateLimitEntry {
    pub(super) fn update_expiration(&mut self, rate_limit: &RateLimit) {
        let expires_at = self.tat.unwrap_or_else(Instant::now) + rate_limit.period;
        self.expires_at = Some(tokio::time::Instant::from_std(expires_at));
    }
}

/// We want to store an in memory cache
#[derive(Clone, Default)]
pub(super) struct InMemoryUpstream {}

impl<Key, Data: Default> LoadFromUpstream<Key, Data> for InMemoryUpstream {
    fn load(&mut self, request: DataLoadRequest<Key, Data>) {
        // if not in the cache, create a new entry
        request.resolve(Default::default());
    }
}

impl<Key, Data: Default> CommitToUpstream<Key, Data> for InMemoryUpstream {
    fn commit<'a>(&mut self, request: DataCommitRequest<'a, Key, Data>) {
        // NOOP: there's no upstream
        request.resolve();
    }
}
