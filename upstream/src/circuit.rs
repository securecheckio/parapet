use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for a single upstream RPC endpoint.
#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicUsize>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    failure_threshold: usize,
    timeout_duration: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, timeout_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            failure_threshold,
            timeout_duration: Duration::from_secs(timeout_secs),
        }
    }

    pub async fn call_permitted(&self) -> bool {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.lock().await;
                if let Some(last_time) = *last_failure {
                    if Instant::now().duration_since(last_time) > self.timeout_duration {
                        log::info!("Circuit breaker: transitioning to HALF_OPEN");
                        *state = CircuitState::HalfOpen;
                        drop(state);
                        drop(last_failure);
                        self.failure_count.store(0, Ordering::SeqCst);
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::HalfOpen => {
                log::info!("Circuit breaker: service recovered, CLOSED");
                *state = CircuitState::Closed;
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    pub async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => {
                if failures >= self.failure_threshold {
                    log::error!("Circuit breaker: OPEN (failures: {})", failures);
                    *state = CircuitState::Open;
                    let mut last_failure = self.last_failure_time.lock().await;
                    *last_failure = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                log::warn!("Circuit breaker: failed in HALF_OPEN, returning to OPEN");
                *state = CircuitState::Open;
                let mut last_failure = self.last_failure_time.lock().await;
                *last_failure = Some(Instant::now());
            }
            _ => {}
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        *self.state.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
