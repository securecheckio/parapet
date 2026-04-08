use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for consent policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentPolicyConfig {
    pub rule_management: RuleManagementConfig,
    pub consent: ConsentConfig,
    #[serde(default)]
    pub rule_manager: Vec<RuleManagerPermissions>,
    #[serde(default)]
    pub consent_policy: Vec<WalletConsentPolicy>,
    #[serde(default)]
    pub notification_provider: Vec<NotificationProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleManagementConfig {
    pub enabled: bool,
    pub api_port: u16,
    pub api_host: String,
    pub authorized_rule_managers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleManagerPermissions {
    pub wallet: String,
    pub can_create_rules: bool,
    pub can_delete_rules: bool,
    pub can_modify_static: bool,
    pub max_dynamic_rules: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentConfig {
    pub enabled: bool,
    pub default_require_consent_above: u32,
    pub default_hard_block_above: u32,
    pub websocket: WebSocketConfig,
    pub notifications: NotificationsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConsentPolicy {
    pub wallet: String,
    pub allowed_approvers: Vec<String>,
    pub require_consent_above: u32,
    pub hard_block_above: u32,
    pub allow_self_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

impl ConsentPolicyConfig {
    /// Load from TOML file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ConsentPolicyConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Get policy for a wallet
    pub fn get_wallet_policy(&self, wallet: &str) -> Option<&WalletConsentPolicy> {
        self.consent_policy.iter().find(|p| p.wallet == wallet)
    }
    
    /// Get default thresholds
    pub fn get_default_thresholds(&self) -> (u32, u32) {
        (
            self.consent.default_require_consent_above,
            self.consent.default_hard_block_above,
        )
    }
    
    /// Check if a wallet is an authorized rule manager
    pub fn is_authorized_rule_manager(&self, wallet: &str) -> bool {
        self.rule_management.authorized_rule_managers.contains(&wallet.to_string())
    }
    
    /// Get rule manager permissions
    pub fn get_rule_manager_permissions(&self, wallet: &str) -> Option<&RuleManagerPermissions> {
        self.rule_manager.iter().find(|rm| rm.wallet == wallet)
    }
}

impl Default for ConsentPolicyConfig {
    fn default() -> Self {
        Self {
            rule_management: RuleManagementConfig {
                enabled: true,
                api_port: 3001,
                api_host: "0.0.0.0".to_string(),
                authorized_rule_managers: vec![],
            },
            consent: ConsentConfig {
                enabled: true,
                default_require_consent_above: 70,
                default_hard_block_above: 90,
                websocket: WebSocketConfig {
                    enabled: true,
                    path: "/ws/escalations".to_string(),
                },
                notifications: NotificationsConfig {
                    enabled: false,
                },
            },
            rule_manager: vec![],
            consent_policy: vec![],
            notification_provider: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ConsentPolicyConfig::default();
        assert!(config.rule_management.enabled);
        assert_eq!(config.rule_management.api_port, 3001);
        assert_eq!(config.consent.default_require_consent_above, 70);
        assert_eq!(config.consent.default_hard_block_above, 90);
    }
}
