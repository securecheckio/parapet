use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub enum FlowbitValue {
    Boolean {
        value: bool,
        set_at: SystemTime,
        expires_at: Option<SystemTime>,
    },
    Counter {
        value: u64,
        set_at: SystemTime,
        expires_at: Option<SystemTime>,
    },
    Timestamp {
        value: SystemTime,
        set_at: SystemTime,
        expires_at: Option<SystemTime>,
    },
}

pub struct FlowbitStateManager {
    wallet_states: HashMap<Pubkey, HashMap<String, FlowbitValue>>,
    global_state: HashMap<String, FlowbitValue>,
    last_cleanup: SystemTime,

    // Configuration
    max_wallets: Option<usize>,
    cleanup_interval: Duration,
    default_ttl: Duration,
}

impl FlowbitStateManager {
    pub fn new(max_wallets: Option<usize>) -> Self {
        // Read env var ONCE at construction (not on every call)
        let default_ttl = std::env::var("SOLSHIELD_FLOWBITS_DEFAULT_TTL")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(300)); // 5 min hardcoded fallback

        log::info!("Flowbits default TTL: {:?}", default_ttl);

        Self {
            wallet_states: HashMap::new(),
            global_state: HashMap::new(),
            last_cleanup: SystemTime::now(),
            max_wallets,
            cleanup_interval: Duration::from_secs(60),
            default_ttl,
        }
    }

    /// Resolve TTL: use provided or fall back to default
    fn resolve_ttl(&self, ttl: Option<Duration>) -> Duration {
        ttl.unwrap_or(self.default_ttl)
    }

    pub fn set(&mut self, wallet: &Pubkey, name: &str, ttl: Option<Duration>) {
        // Check memory limit
        if let Some(max) = self.max_wallets {
            if self.wallet_states.len() >= max && !self.wallet_states.contains_key(wallet) {
                log::warn!(
                    "Flowbit memory limit reached ({} wallets), evicting oldest",
                    max
                );
                self.evict_oldest();
            }
        }

        let now = SystemTime::now();
        let effective_ttl = self.resolve_ttl(ttl);
        let expires_at = Some(now + effective_ttl);

        let state = self.wallet_states.entry(*wallet).or_default();
        state.insert(
            name.to_string(),
            FlowbitValue::Boolean {
                value: true,
                set_at: now,
                expires_at,
            },
        );
    }

    /// Check if flowbit is set AND not expired (atomic check)
    pub fn is_set(&self, wallet: &Pubkey, name: &str) -> bool {
        if let Some(state) = self.wallet_states.get(wallet) {
            if let Some(value) = state.get(name) {
                return !self.is_expired(value);
            }
        }
        false
    }

    /// Check if flowbit is set AND was set within the specified time window (atomic check)
    pub fn is_set_within(&self, wallet: &Pubkey, name: &str, within_seconds: u64) -> bool {
        if let Some(state) = self.wallet_states.get(wallet) {
            if let Some(value) = state.get(name) {
                if self.is_expired(value) {
                    return false;
                }
                
                // Check if set_at is within the time window
                let set_at = match value {
                    FlowbitValue::Boolean { set_at, .. } => set_at,
                    FlowbitValue::Counter { set_at, .. } => set_at,
                    FlowbitValue::Timestamp { set_at, .. } => set_at,
                };
                
                let now = SystemTime::now();
                if let Ok(elapsed) = now.duration_since(*set_at) {
                    return elapsed.as_secs() <= within_seconds;
                }
            }
        }
        false
    }

    pub fn increment(&mut self, wallet: &Pubkey, name: &str, ttl: Option<Duration>) {
        // Check memory limit
        if let Some(max) = self.max_wallets {
            if self.wallet_states.len() >= max && !self.wallet_states.contains_key(wallet) {
                log::warn!("Flowbit memory limit reached, evicting oldest");
                self.evict_oldest();
            }
        }

        let now = SystemTime::now();
        
        // Check if counter exists and is not expired first
        let new_value = if let Some(state) = self.wallet_states.get(wallet) {
            if let Some(existing) = state.get(name) {
                match existing {
                    FlowbitValue::Counter { value, .. } if !self.is_expired(existing) => value + 1,
                    _ => 1,
                }
            } else {
                1
            }
        } else {
            1
        };

        // Now mutate
        let effective_ttl = self.resolve_ttl(ttl);
        let expires_at = Some(now + effective_ttl);
        
        let state = self.wallet_states.entry(*wallet).or_default();
        state.insert(
            name.to_string(),
            FlowbitValue::Counter {
                value: new_value,
                set_at: now,
                expires_at,
            },
        );
    }

    pub fn get_counter(&self, wallet: &Pubkey, name: &str) -> u64 {
        if let Some(state) = self.wallet_states.get(wallet) {
            if let Some(value) = state.get(name) {
                if let FlowbitValue::Counter { value: count, .. } = value {
                    if !self.is_expired(value) {
                        return *count;
                    }
                }
            }
        }
        0
    }

    pub fn unset(&mut self, wallet: &Pubkey, name: &str) {
        if let Some(state) = self.wallet_states.get_mut(wallet) {
            state.remove(name);
        }
    }

    // ===== GLOBAL FLOWBIT OPERATIONS =====

    pub fn set_global(&mut self, name: &str, ttl: Option<Duration>) {
        let now = SystemTime::now();
        let effective_ttl = self.resolve_ttl(ttl);
        let expires_at = Some(now + effective_ttl);

        self.global_state.insert(
            name.to_string(),
            FlowbitValue::Boolean {
                value: true,
                set_at: now,
                expires_at,
            },
        );
    }

    pub fn is_set_global(&self, name: &str) -> bool {
        if let Some(value) = self.global_state.get(name) {
            return !self.is_expired(value);
        }
        false
    }

    pub fn is_set_within_global(&self, name: &str, within_seconds: u64) -> bool {
        if let Some(value) = self.global_state.get(name) {
            if self.is_expired(value) {
                return false;
            }
            
            let set_at = match value {
                FlowbitValue::Boolean { set_at, .. } => set_at,
                FlowbitValue::Counter { set_at, .. } => set_at,
                FlowbitValue::Timestamp { set_at, .. } => set_at,
            };
            
            let now = SystemTime::now();
            if let Ok(elapsed) = now.duration_since(*set_at) {
                return elapsed.as_secs() <= within_seconds;
            }
        }
        false
    }

    pub fn increment_global(&mut self, name: &str, ttl: Option<Duration>) {
        let now = SystemTime::now();
        
        // Check if counter exists and is not expired first
        let new_value = if let Some(existing) = self.global_state.get(name) {
            match existing {
                FlowbitValue::Counter { value, .. } if !self.is_expired(existing) => value + 1,
                _ => 1,
            }
        } else {
            1
        };

        // Now mutate
        let effective_ttl = self.resolve_ttl(ttl);
        let expires_at = Some(now + effective_ttl);
        
        self.global_state.insert(
            name.to_string(),
            FlowbitValue::Counter {
                value: new_value,
                set_at: now,
                expires_at,
            },
        );
    }

    pub fn get_counter_global(&self, name: &str) -> u64 {
        if let Some(value) = self.global_state.get(name) {
            if let FlowbitValue::Counter { value: count, .. } = value {
                if !self.is_expired(value) {
                    return *count;
                }
            }
        }
        0
    }

    pub fn unset_global(&mut self, name: &str) {
        self.global_state.remove(name);
    }

    fn is_expired(&self, value: &FlowbitValue) -> bool {
        let expires_at = match value {
            FlowbitValue::Boolean { expires_at, .. } => expires_at,
            FlowbitValue::Counter { expires_at, .. } => expires_at,
            FlowbitValue::Timestamp { expires_at, .. } => expires_at,
        };

        if let Some(exp) = expires_at {
            SystemTime::now() > *exp
        } else {
            false
        }
    }

    /// Lazy cleanup: Remove expired flowbits for a specific wallet
    #[allow(dead_code)]
    fn cleanup_wallet(&mut self, wallet: &Pubkey) {
        // First, collect expired keys
        let expired_keys: Vec<String> = if let Some(state) = self.wallet_states.get(wallet) {
            state
                .iter()
                .filter_map(|(k, v)| {
                    if self.is_expired(v) {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            return;
        };
        
        // Then remove them
        if let Some(state) = self.wallet_states.get_mut(wallet) {
            for key in expired_keys {
                state.remove(&key);
            }
            
            if state.is_empty() {
                self.wallet_states.remove(wallet);
            }
        }
    }

    /// Background cleanup: Periodically sweep all wallets (called from background task)
    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now();

        // Only run full cleanup every 60 seconds
        if now
            .duration_since(self.last_cleanup)
            .unwrap_or_default()
            < self.cleanup_interval
        {
            return;
        }

        // Clean all wallets - collect expired entries first to avoid borrow issues
        let wallets_to_clean: Vec<Pubkey> = self.wallet_states.keys().copied().collect();
        
        for wallet in wallets_to_clean {
            // First collect expired keys
            let expired_keys: Vec<String> = if let Some(state) = self.wallet_states.get(&wallet) {
                state
                    .iter()
                    .filter_map(|(k, v)| {
                        if self.is_expired(v) {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                continue;
            };
            
            // Then remove them
            if let Some(state) = self.wallet_states.get_mut(&wallet) {
                for key in expired_keys {
                    state.remove(&key);
                }
                
                if state.is_empty() {
                    self.wallet_states.remove(&wallet);
                }
            }
        }

        // Clean global state
        let expired_global_keys: Vec<String> = self.global_state
            .iter()
            .filter_map(|(k, v)| {
                if self.is_expired(v) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        
        for key in expired_global_keys {
            self.global_state.remove(&key);
        }

        self.last_cleanup = now;

        log::debug!(
            "Flowbits cleanup: {} wallets remaining",
            self.wallet_states.len()
        );
    }

    fn evict_oldest(&mut self) {
        // Find wallet with oldest flowbit
        let mut oldest: Option<(Pubkey, SystemTime)> = None;

        for (wallet, state) in &self.wallet_states {
            for flowbit in state.values() {
                if let Some(exp) = self.get_expiration(flowbit) {
                    if oldest.is_none() || Some(exp) < oldest.map(|(_, t)| t) {
                        oldest = Some((*wallet, exp));
                    }
                }
            }
        }

        if let Some((wallet, _)) = oldest {
            self.wallet_states.remove(&wallet);
            log::debug!("Evicted wallet {:?} due to memory limit", wallet);
        }
    }

    fn get_expiration(&self, value: &FlowbitValue) -> Option<SystemTime> {
        match value {
            FlowbitValue::Boolean { expires_at, .. } => *expires_at,
            FlowbitValue::Counter { expires_at, .. } => *expires_at,
            FlowbitValue::Timestamp { expires_at, .. } => *expires_at,
        }
    }

    pub fn memory_usage(&self) -> usize {
        let wallet_count = self.wallet_states.len();
        let avg_flowbits_per_wallet = 3;
        let bytes_per_flowbit = 128;
        wallet_count * avg_flowbits_per_wallet * bytes_per_flowbit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flowbit_set_and_check() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set flowbit
        manager.set(&wallet, "test_flag", Some(Duration::from_secs(60)));

        // Check it's set
        assert!(manager.is_set(&wallet, "test_flag"));

        // Check non-existent flowbit
        assert!(!manager.is_set(&wallet, "nonexistent"));
    }

    #[test]
    fn test_flowbit_expiration() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set flowbit with very short TTL
        manager.set(&wallet, "short_lived", Some(Duration::from_millis(10)));

        // Should be set immediately
        assert!(manager.is_set(&wallet, "short_lived"));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Should be expired
        assert!(!manager.is_set(&wallet, "short_lived"));
    }

    #[test]
    fn test_counter_increment() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Increment counter
        manager.increment(&wallet, "counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter(&wallet, "counter"), 1);

        // Increment again
        manager.increment(&wallet, "counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter(&wallet, "counter"), 2);

        // Increment third time
        manager.increment(&wallet, "counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter(&wallet, "counter"), 3);
    }

    #[test]
    fn test_memory_limit() {
        let mut manager = FlowbitStateManager::new(Some(2)); // Max 2 wallets

        let wallet1 = Pubkey::new_unique();
        let wallet2 = Pubkey::new_unique();
        let wallet3 = Pubkey::new_unique();

        // Add 2 wallets
        manager.set(&wallet1, "flag1", Some(Duration::from_secs(60)));
        manager.set(&wallet2, "flag2", Some(Duration::from_secs(60)));

        assert_eq!(manager.wallet_states.len(), 2);

        // Add 3rd wallet - should evict oldest
        manager.set(&wallet3, "flag3", Some(Duration::from_secs(60)));

        // Should still be 2 wallets
        assert_eq!(manager.wallet_states.len(), 2);

        // wallet1 should be evicted (oldest)
        assert!(!manager.is_set(&wallet1, "flag1"));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set flowbit with short TTL
        manager.set(&wallet, "short", Some(Duration::from_millis(10)));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Run cleanup
        manager.last_cleanup = SystemTime::now() - Duration::from_secs(120); // Force cleanup
        manager.cleanup_expired();

        // Wallet should be removed (no flowbits left)
        assert_eq!(manager.wallet_states.len(), 0);
    }

    #[test]
    fn test_unset() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set flowbit
        manager.set(&wallet, "flag", Some(Duration::from_secs(60)));
        assert!(manager.is_set(&wallet, "flag"));

        // Unset it
        manager.unset(&wallet, "flag");
        assert!(!manager.is_set(&wallet, "flag"));
    }

    #[test]
    fn test_default_ttl() {
        let manager = FlowbitStateManager::new(None);

        // Should use default TTL (5 minutes)
        assert_eq!(manager.default_ttl, Duration::from_secs(300));
    }

    #[test]
    fn test_within_seconds() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set flowbit
        manager.set(&wallet, "recent", Some(Duration::from_secs(60)));

        // Should be set within 5 seconds
        assert!(manager.is_set_within(&wallet, "recent", 5));

        // Wait 2 seconds
        std::thread::sleep(Duration::from_secs(2));

        // Should NOT be set within 1 second (2 seconds have passed)
        assert!(!manager.is_set_within(&wallet, "recent", 1));

        // Should still be set within 5 seconds
        assert!(manager.is_set_within(&wallet, "recent", 5));
    }

    #[test]
    fn test_within_seconds_expired() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set flowbit with very short TTL
        manager.set(&wallet, "short", Some(Duration::from_millis(10)));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Should NOT be set within any time window (expired)
        assert!(!manager.is_set_within(&wallet, "short", 100));
    }

    // ===== GLOBAL FLOWBIT TESTS =====

    #[test]
    fn test_global_set_and_check() {
        let mut manager = FlowbitStateManager::new(None);

        // Set global flowbit
        manager.set_global("global_flag", Some(Duration::from_secs(60)));

        // Check it's set
        assert!(manager.is_set_global("global_flag"));

        // Check non-existent flowbit
        assert!(!manager.is_set_global("nonexistent"));
    }

    #[test]
    fn test_global_increment() {
        let mut manager = FlowbitStateManager::new(None);

        // Increment global counter
        manager.increment_global("global_counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter_global("global_counter"), 1);

        // Increment again
        manager.increment_global("global_counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter_global("global_counter"), 2);

        // Increment third time
        manager.increment_global("global_counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter_global("global_counter"), 3);
    }

    #[test]
    fn test_global_vs_per_wallet_independence() {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();

        // Set per-wallet counter
        manager.increment(&wallet, "counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter(&wallet, "counter"), 1);

        // Set global counter with same name
        manager.increment_global("counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter_global("counter"), 1);

        // They should be independent
        assert_eq!(manager.get_counter(&wallet, "counter"), 1);
        assert_eq!(manager.get_counter_global("counter"), 1);

        // Increment per-wallet
        manager.increment(&wallet, "counter", Some(Duration::from_secs(60)));
        assert_eq!(manager.get_counter(&wallet, "counter"), 2);
        assert_eq!(manager.get_counter_global("counter"), 1); // Global unchanged
    }

    #[test]
    fn test_global_expiration() {
        let mut manager = FlowbitStateManager::new(None);

        // Set global flowbit with short TTL
        manager.set_global("short_lived", Some(Duration::from_millis(10)));
        assert!(manager.is_set_global("short_lived"));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Should be expired
        assert!(!manager.is_set_global("short_lived"));
    }

    #[test]
    fn test_global_within_seconds() {
        let mut manager = FlowbitStateManager::new(None);

        // Set global flowbit
        manager.set_global("recent", Some(Duration::from_secs(60)));

        // Should be set within 5 seconds
        assert!(manager.is_set_within_global("recent", 5));

        // Wait 2 seconds
        std::thread::sleep(Duration::from_secs(2));

        // Should NOT be set within 1 second (2 seconds have passed)
        assert!(!manager.is_set_within_global("recent", 1));

        // Should still be set within 5 seconds
        assert!(manager.is_set_within_global("recent", 5));
    }

    #[test]
    fn test_global_unset() {
        let mut manager = FlowbitStateManager::new(None);

        // Set global flowbit
        manager.set_global("flag", Some(Duration::from_secs(60)));
        assert!(manager.is_set_global("flag"));

        // Unset it
        manager.unset_global("flag");
        assert!(!manager.is_set_global("flag"));
    }

    #[test]
    fn test_lateral_movement_scenario() {
        let mut manager = FlowbitStateManager::new(None);

        let recipient = "7xK...9mP";
        
        // Simulate 3 different wallets sending to same recipient
        manager.increment_global(&format!("suspicious_recipient:{}", recipient), Some(Duration::from_secs(3600)));
        assert_eq!(manager.get_counter_global(&format!("suspicious_recipient:{}", recipient)), 1);

        manager.increment_global(&format!("suspicious_recipient:{}", recipient), Some(Duration::from_secs(3600)));
        assert_eq!(manager.get_counter_global(&format!("suspicious_recipient:{}", recipient)), 2);

        manager.increment_global(&format!("suspicious_recipient:{}", recipient), Some(Duration::from_secs(3600)));
        assert_eq!(manager.get_counter_global(&format!("suspicious_recipient:{}", recipient)), 3);

        // Should trigger lateral movement detection (> 2)
        assert!(manager.get_counter_global(&format!("suspicious_recipient:{}", recipient)) > 2);
    }
}
