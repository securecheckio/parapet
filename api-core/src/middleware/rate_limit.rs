use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

/// Per-API-key rate limiter for MCP endpoints
#[derive(Clone)]
pub struct McpRateLimiter {
    /// Per-API-key quotas (scans per hour)
    quotas: Arc<Mutex<HashMap<String, ApiKeyQuota>>>,
    /// Global concurrent scan limiter (prevent API quota exhaustion)
    concurrent_scans: Arc<Semaphore>,
}

struct ApiKeyQuota {
    scans_this_hour: u32,
    hour_started: Instant,
    max_scans_per_hour: u32,
}

impl McpRateLimiter {
    pub fn new(max_concurrent_scans: usize, scans_per_hour_per_key: u32) -> Self {
        log::info!(
            "🚦 MCP rate limiter initialized: {} concurrent scans, {} scans/hour per API key",
            max_concurrent_scans,
            scans_per_hour_per_key
        );

        Self {
            quotas: Arc::new(Mutex::new(HashMap::new())),
            concurrent_scans: Arc::new(Semaphore::new(max_concurrent_scans)),
        }
    }

    /// Check and acquire quota for an API key
    /// Returns Ok(permit) if allowed, Err if quota exceeded
    pub async fn check_quota(&self, api_key: &str) -> Result<QuotaPermit, RateLimitError> {
        let mut quotas = self.quotas.lock().await;

        let quota = quotas
            .entry(api_key.to_string())
            .or_insert_with(|| ApiKeyQuota {
                scans_this_hour: 0,
                hour_started: Instant::now(),
                max_scans_per_hour: 10, // Default: 10 scans/hour
            });

        // Reset quota if hour has elapsed
        if quota.hour_started.elapsed() >= Duration::from_secs(3600) {
            quota.scans_this_hour = 0;
            quota.hour_started = Instant::now();
        }

        // Check quota
        if quota.scans_this_hour >= quota.max_scans_per_hour {
            let time_until_reset = Duration::from_secs(3600)
                .checked_sub(quota.hour_started.elapsed())
                .unwrap_or(Duration::ZERO);

            return Err(RateLimitError::QuotaExceeded {
                limit: quota.max_scans_per_hour,
                reset_in_seconds: time_until_reset.as_secs(),
            });
        }

        // Increment usage
        quota.scans_this_hour += 1;

        // Try to acquire concurrent scan permit (non-blocking check)
        let concurrent_permit = self
            .concurrent_scans
            .clone()
            .try_acquire_owned()
            .map_err(|_| RateLimitError::TooManyConcurrentScans {
                max_concurrent: self.concurrent_scans.available_permits() + 1,
            })?;

        Ok(QuotaPermit {
            _concurrent_permit: concurrent_permit,
            scans_remaining: quota.max_scans_per_hour - quota.scans_this_hour,
            reset_in_seconds: 3600 - quota.hour_started.elapsed().as_secs(),
        })
    }

}

pub struct QuotaPermit {
    _concurrent_permit: tokio::sync::OwnedSemaphorePermit,
    pub scans_remaining: u32,
    pub reset_in_seconds: u64,
}

#[derive(Debug)]
pub enum RateLimitError {
    QuotaExceeded {
        limit: u32,
        reset_in_seconds: u64,
    },
    TooManyConcurrentScans {
        max_concurrent: usize,
    },
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        match self {
            RateLimitError::QuotaExceeded {
                limit,
                reset_in_seconds,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "quota_exceeded",
                    "message": format!("API key quota exceeded: {} scans per hour", limit),
                    "limit": limit,
                    "reset_in_seconds": reset_in_seconds,
                    "reset_in_minutes": reset_in_seconds / 60
                })),
            )
                .into_response(),
            RateLimitError::TooManyConcurrentScans { max_concurrent } => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "too_many_concurrent_scans",
                    "message": format!("Maximum {} concurrent scans allowed. Please try again in a moment.", max_concurrent),
                    "max_concurrent": max_concurrent
                })),
            )
                .into_response(),
        }
    }
}
