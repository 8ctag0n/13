use governor::{Quota, RateLimiter};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use std::num::NonZeroU32;
use std::sync::Arc;

pub type GlobalRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a global rate limiter
pub fn create_rate_limiter(requests_per_minute: u32) -> GlobalRateLimiter {
    let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
    Arc::new(RateLimiter::direct(quota))
}

/// Check if request should be rate limited
pub fn check_rate_limit(limiter: &GlobalRateLimiter) -> bool {
    limiter.check().is_ok()
}
