use std::ops::Deref;

use crate::{
    clock::{Clock, InstantClock},
    GcraError, GcraState, RateLimit,
};

/// A simple wrapper to help make using [RateLimit]s with [GcraState]s easier for basic cases.
pub struct RateLimitGuard<C: Clock = InstantClock> {
    clock: C,
    rate_limit: RateLimit,
    state: GcraState,
}

impl RateLimitGuard {
    pub fn new_state(rate_limit: RateLimit) -> Self {
        RateLimitGuard {
            clock: InstantClock,
            rate_limit,
            state: GcraState::default(),
        }
    }
}

impl<C: Clock> RateLimitGuard<C> {
    pub fn new(clock: C, rate_limit: RateLimit, state: GcraState) -> Self {
        RateLimitGuard {
            clock,
            rate_limit,
            state,
        }
    }

    /// Check if we are allowed to proceed. If so updated our internal state and return true.
    pub fn check_and_modify(&mut self, cost: u32) -> Result<(), GcraError> {
        let RateLimitGuard {
            clock,
            rate_limit,
            state,
        } = self;
        let arrived_at = clock.now();
        state.check_and_modify_at(rate_limit, arrived_at, cost)
    }

    /// Get the remaing resources that we have available for the guard at the current moment in time.
    pub fn remaining_resources(&self) -> u32 {
        self.state
            .remaining_resources(&self.rate_limit, self.clock.now())
    }

    /// Reverts rate_limit by cost, and update our internal state.
    pub fn revert(&mut self, cost: u32) -> Result<(), GcraError> {
        let RateLimitGuard {
            clock,
            rate_limit,
            state,
        } = self;
        let arrived_at = clock.now();
        state.revert_at(rate_limit, arrived_at, cost)
    }
}

impl<C: Clock> Deref for RateLimitGuard<C> {
    type Target = GcraState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
