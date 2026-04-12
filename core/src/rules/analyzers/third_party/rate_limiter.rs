use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Instant};

/// Shared rate limiter for third-party API analyzers
/// Uses a token bucket algorithm with automatic refill
#[derive(Clone)]
pub struct ApiRateLimiter {
    semaphore: Arc<Semaphore>,
    requests_per_window: usize,
    window_duration: Duration,
    last_refill: Arc<tokio::sync::Mutex<Instant>>,
}

impl ApiRateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `requests_per_window` - Maximum requests allowed in the time window
    /// * `window_duration` - Duration of the rate limit window
    ///
    /// # Example
    /// ```
    /// // Jupiter free tier: 60 requests per 60 seconds
    /// let limiter = ApiRateLimiter::new(60, Duration::from_secs(60));
    ///
    /// // Helius: 10,000 requests per day (conservative: ~100/min)
    /// let limiter = ApiRateLimiter::new(100, Duration::from_secs(60));
    /// ```
    pub fn new(requests_per_window: usize, window_duration: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(requests_per_window)),
            requests_per_window,
            window_duration,
            last_refill: Arc::new(tokio::sync::Mutex::new(Instant::now())),
        }
    }

    /// Create from environment variable or use default
    ///
    /// Expects format: "REQUESTS/SECONDS" (e.g., "60/60" or "100/10")
    /// If not set or invalid, uses the provided defaults
    pub fn from_env_or_default(
        env_var: &str,
        default_requests: usize,
        default_window_secs: u64,
    ) -> Self {
        let (requests, window_secs) = std::env::var(env_var)
            .ok()
            .and_then(|val| {
                let parts: Vec<&str> = val.split('/').collect();
                if parts.len() == 2 {
                    let requests = parts[0].parse().ok()?;
                    let seconds = parts[1].parse().ok()?;
                    Some((requests, seconds))
                } else {
                    None
                }
            })
            .unwrap_or((default_requests, default_window_secs));

        let limiter = Self::new(requests, Duration::from_secs(window_secs));

        log::info!(
            "🚦 Rate limiter configured: {} requests per {} seconds (env: {})",
            requests,
            window_secs,
            env_var
        );

        limiter
    }

    /// Acquire permission to make an API call
    /// Blocks until a permit is available or refill occurs
    pub async fn acquire(&self) -> RateLimitPermit {
        // Check if we need to refill permits
        self.try_refill().await;

        // Wait for a permit (blocks if none available)
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Semaphore closed");

        RateLimitPermit {
            _permit: permit,
            acquired_at: Instant::now(),
        }
    }

    /// Try to acquire without blocking
    /// Returns None if no permits available
    pub async fn try_acquire(&self) -> Option<RateLimitPermit> {
        self.try_refill().await;

        self.semaphore
            .clone()
            .try_acquire_owned()
            .ok()
            .map(|permit| RateLimitPermit {
                _permit: permit,
                acquired_at: Instant::now(),
            })
    }

    /// Refill permits if window has elapsed
    async fn try_refill(&self) {
        let mut last_refill = self.last_refill.lock().await;
        let elapsed = last_refill.elapsed();

        if elapsed >= self.window_duration {
            // Refill all permits
            let available = self.semaphore.available_permits();
            let to_add = self.requests_per_window.saturating_sub(available);

            if to_add > 0 {
                self.semaphore.add_permits(to_add);
                log::debug!(
                    "🔄 Rate limiter refilled: {} permits added (window elapsed: {:?})",
                    to_add,
                    elapsed
                );
            }

            *last_refill = Instant::now();
        }
    }

    /// Get current available permits (for monitoring)
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Wait with exponential backoff on rate limit errors
    pub async fn backoff_on_429(attempt: u32) {
        let delay_ms = 1000 * 2_u64.pow(attempt.min(5)); // Max 32 second delay
        log::warn!(
            "⏳ Rate limited (429) - backing off for {}ms (attempt {})",
            delay_ms,
            attempt
        );
        sleep(Duration::from_millis(delay_ms)).await;
    }
}

/// RAII permit that returns to the pool when dropped
pub struct RateLimitPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    acquired_at: Instant,
}

impl RateLimitPermit {
    /// How long this permit has been held
    pub fn held_duration(&self) -> Duration {
        self.acquired_at.elapsed()
    }
}

impl Drop for RateLimitPermit {
    fn drop(&mut self) {
        log::trace!(
            "🔓 Rate limit permit released (held: {:?})",
            self.held_duration()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = ApiRateLimiter::new(5, Duration::from_secs(1));

        // Hold all five permits (do not drop between iterations).
        let mut permits = Vec::with_capacity(5);
        for i in 0..5 {
            permits.push(limiter.acquire().await);
            assert_eq!(limiter.available_permits(), 4 - i);
        }

        assert_eq!(limiter.available_permits(), 0);
        drop(permits);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = ApiRateLimiter::new(2, Duration::from_millis(100));

        // Use all permits
        let _p1 = limiter.acquire().await;
        let _p2 = limiter.acquire().await;
        assert_eq!(limiter.available_permits(), 0);

        // Wait for refill
        sleep(Duration::from_millis(150)).await;

        // Should have permits again
        let _p3 = limiter.acquire().await;
        assert!(limiter.available_permits() >= 1);
    }

    #[tokio::test]
    async fn test_try_acquire() {
        let limiter = ApiRateLimiter::new(1, Duration::from_secs(10));

        // First should succeed
        let _p1 = limiter.try_acquire().await;
        assert!(_p1.is_some());

        // Second should fail (no permits)
        let p2 = limiter.try_acquire().await;
        assert!(p2.is_none());
    }
}
