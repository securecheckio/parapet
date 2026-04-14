use crate::rpc_handler::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep, Duration, Instant};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for upstream RPC
struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicUsize>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    failure_threshold: usize,
    timeout_duration: Duration,
}

impl CircuitBreaker {
    fn new(failure_threshold: usize, timeout_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            failure_threshold,
            timeout_duration: Duration::from_secs(timeout_secs),
        }
    }

    async fn call_permitted(&self) -> bool {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure_time.lock().await;
                if let Some(last_time) = *last_failure {
                    if Instant::now().duration_since(last_time) > self.timeout_duration {
                        log::info!("🔄 Circuit breaker: Transitioning to HALF_OPEN");
                        *state = CircuitState::HalfOpen;
                        drop(state);
                        drop(last_failure);
                        self.failure_count.store(0, Ordering::SeqCst);
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                // Allow limited calls in half-open state
                true
            }
        }
    }

    async fn record_success(&self) {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::HalfOpen => {
                log::info!("✅ Circuit breaker: Service recovered, transitioning to CLOSED");
                *state = CircuitState::Closed;
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => {
                if failures >= self.failure_threshold {
                    log::error!("🚨 Circuit breaker: OPEN (failures: {})", failures);
                    *state = CircuitState::Open;
                    let mut last_failure = self.last_failure_time.lock().await;
                    *last_failure = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                log::warn!("⚠️  Circuit breaker: Failed in HALF_OPEN, returning to OPEN");
                *state = CircuitState::Open;
                let mut last_failure = self.last_failure_time.lock().await;
                *last_failure = Some(Instant::now());
            }
            _ => {}
        }
    }

    async fn get_state(&self) -> CircuitState {
        *self.state.lock().await
    }
}

pub struct UpstreamClient {
    client: Client,
    upstream_url: String,
    /// Semaphore to limit concurrent requests to upstream
    concurrency_limiter: Arc<Semaphore>,
    /// Minimum delay between requests (milliseconds)
    request_delay_ms: u64,
    /// Circuit breaker to prevent cascading failures
    circuit_breaker: CircuitBreaker,
    /// Retry configuration
    max_retries: usize,
    retry_base_delay_ms: u64,
}

impl UpstreamClient {
    pub fn new(upstream_url: String) -> Self {
        // Default: max 10 concurrent requests, 100ms delay between requests
        Self::new_with_config(upstream_url, UpstreamConfig::default())
    }

    /// Create a new upstream client with custom rate limits (backwards compatible)
    pub fn new_with_limits(upstream_url: String, max_concurrent: usize, delay_ms: u64) -> Self {
        Self::new_with_config(
            upstream_url,
            UpstreamConfig {
                max_concurrent,
                delay_ms,
                ..Default::default()
            },
        )
    }

    /// Create a new upstream client with full configuration
    pub fn new_with_config(upstream_url: String, config: UpstreamConfig) -> Self {
        log::info!(
            "🚦 Upstream config: max {} concurrent, {}ms delay, timeout {}s, retries {}, circuit breaker threshold {}",
            config.max_concurrent,
            config.delay_ms,
            config.timeout_secs,
            config.max_retries,
            config.circuit_breaker_threshold
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            upstream_url,
            concurrency_limiter: Arc::new(Semaphore::new(config.max_concurrent)),
            request_delay_ms: config.delay_ms,
            circuit_breaker: CircuitBreaker::new(
                config.circuit_breaker_threshold,
                config.circuit_breaker_timeout_secs,
            ),
            max_retries: config.max_retries,
            retry_base_delay_ms: config.retry_base_delay_ms,
        }
    }

    pub async fn forward(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        log::debug!("📤 Forwarding to upstream: method={}", request.method);

        // Check circuit breaker
        if !self.circuit_breaker.call_permitted().await {
            let state = self.circuit_breaker.get_state().await;
            return Err(anyhow::anyhow!(
                "Circuit breaker is {:?} - upstream service is unavailable",
                state
            ));
        }

        // Acquire semaphore permit to limit concurrent requests
        let _permit = self.concurrency_limiter.acquire().await?;
        log::debug!("🎫 Acquired upstream request permit");

        // Add delay to throttle request rate
        if self.request_delay_ms > 0 {
            sleep(Duration::from_millis(self.request_delay_ms)).await;
        }

        // Retry loop with exponential backoff
        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= self.max_retries {
            if attempt > 0 {
                let backoff_ms = self.retry_base_delay_ms * 2u64.pow(attempt as u32 - 1);
                log::debug!(
                    "🔄 Retry attempt {} after {}ms backoff",
                    attempt,
                    backoff_ms
                );
                sleep(Duration::from_millis(backoff_ms)).await;
            }

            match self.try_request(request).await {
                Ok(response) => {
                    // Success - record in circuit breaker
                    self.circuit_breaker.record_success().await;
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    attempt += 1;

                    // Check if error is retryable
                    if let Some(err) = &last_error {
                        if !Self::is_retryable_error(err) {
                            log::debug!("❌ Non-retryable error, not retrying");
                            break;
                        }
                    }
                }
            }
        }

        // All retries failed - record failure in circuit breaker
        self.circuit_breaker.record_failure().await;

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("Request failed after {} attempts", attempt)))
    }

    /// Try a single request (no retries)
    async fn try_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let response = self
            .client
            .post(&self.upstream_url)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Upstream returned error: status={}, body={}",
                status,
                body
            ));
        }

        let rpc_response: JsonRpcResponse = response.json().await?;
        log::debug!("📥 Received response from upstream");

        Ok(rpc_response)
    }

    /// Check if an error is retryable (network errors, timeouts, 5xx errors)
    fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();

        // Network errors
        if error_str.contains("connection")
            || error_str.contains("timeout")
            || error_str.contains("network")
            || error_str.contains("dns")
        {
            return true;
        }

        // 5xx server errors
        if error_str.contains("status=5") {
            return true;
        }

        // 429 rate limit (should retry with backoff)
        if error_str.contains("status=429") {
            return true;
        }

        // 4xx client errors (except 429) are not retryable
        false
    }

    /// Get circuit breaker state for monitoring
    pub async fn get_circuit_state(&self) -> CircuitState {
        self.circuit_breaker.get_state().await
    }
}

/// Configuration for upstream client
#[derive(Debug, Clone)]
pub struct UpstreamConfig {
    pub max_concurrent: usize,
    pub delay_ms: u64,
    pub timeout_secs: u64,
    pub max_retries: usize,
    pub retry_base_delay_ms: u64,
    pub circuit_breaker_threshold: usize,
    pub circuit_breaker_timeout_secs: u64,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            delay_ms: 100,
            timeout_secs: 30,
            max_retries: 3,
            retry_base_delay_ms: 100,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout_secs: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_error_classification_covers_expected_statuses() {
        assert!(UpstreamClient::is_retryable_error(&anyhow::anyhow!("status=500")));
        assert!(UpstreamClient::is_retryable_error(&anyhow::anyhow!("status=429")));
        assert!(UpstreamClient::is_retryable_error(&anyhow::anyhow!("connection reset")));
        assert!(!UpstreamClient::is_retryable_error(&anyhow::anyhow!("status=400")));
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(2, 60);
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        assert!(!cb.call_permitted().await);
    }

    #[tokio::test]
    async fn circuit_breaker_half_open_and_recovery() {
        let cb = CircuitBreaker::new(1, 0);
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        assert!(cb.call_permitted().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
        cb.record_success().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
}
