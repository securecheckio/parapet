use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone)]
pub struct PlatformConfig {
    pub database_url: String,
    pub frontend_url: String,
    pub frontend_cors_credentials: bool,
    pub push_notifications: PushConfig,
    pub payments: PaymentConfig,
    pub learning_enabled: bool,
    pub rules_display_path: String,
}

#[derive(Clone)]
pub struct PushConfig {
    pub enabled: bool,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
}

#[derive(Clone)]
pub struct PaymentConfig {
    pub enabled: bool,
    pub token: TokenInfo,
    pub usdc_mint: String,
    pub treasury_wallet: String,
    pub pricing: PricingTiers,
}

#[derive(Clone)]
pub struct TokenInfo {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub logo: String,
}

#[derive(Clone)]
pub struct PricingTiers {
    pub small_price: u64,
    pub small_credits: i64,
    pub medium_price: u64,
    pub medium_credits: i64,
    pub large_price: u64,
    pub large_credits: i64,
    pub xlarge_price: u64,
    pub xlarge_credits: i64,
}

pub fn load_platform_config_from_file(path: &str) -> Result<PlatformConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read platform config from {}", path))?;
    
    #[derive(Deserialize)]
    struct TomlConfig {
        database: DatabaseConfig,
        frontend: FrontendConfig,
        push_notifications: PushNotificationsConfig,
        payments: PaymentsConfig,
        learning: LearningConfig,
        display: DisplayConfig,
    }
    
    #[derive(Deserialize)]
    struct DatabaseConfig {
        url: String,
    }
    
    #[derive(Deserialize)]
    struct FrontendConfig {
        url: String,
        #[serde(default = "default_true")]
        cors_credentials: bool,
    }
    
    #[derive(Deserialize)]
    struct PushNotificationsConfig {
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        public_key: String,
        #[serde(default)]
        private_key: String,
    }
    
    #[derive(Deserialize)]
    struct PaymentsConfig {
        #[serde(default)]
        enabled: bool,
        #[serde(default = "default_token_mint")]
        token_mint: String,
        #[serde(default = "default_token_name")]
        token_name: String,
        #[serde(default = "default_token_symbol")]
        token_symbol: String,
        #[serde(default = "default_token_decimals")]
        token_decimals: u8,
        #[serde(default = "default_token_logo")]
        token_logo: String,
        #[serde(default = "default_usdc_mint")]
        usdc_mint: String,
        #[serde(default)]
        treasury_wallet: String,
        pricing: PricingToml,
    }
    
    #[derive(Deserialize)]
    struct PricingToml {
        small_price: u64,
        small_credits: i64,
        medium_price: u64,
        medium_credits: i64,
        large_price: u64,
        large_credits: i64,
        xlarge_price: u64,
        xlarge_credits: i64,
    }
    
    #[derive(Deserialize)]
    struct LearningConfig {
        #[serde(default = "default_true")]
        enabled: bool,
    }
    
    #[derive(Deserialize)]
    struct DisplayConfig {
        #[serde(default = "default_rules_path")]
        rules_path: String,
    }
    
    fn default_true() -> bool { true }
    fn default_token_mint() -> String { "7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz".to_string() }
    fn default_token_name() -> String { "xLABS".to_string() }
    fn default_token_symbol() -> String { "xLABS".to_string() }
    fn default_token_decimals() -> u8 { 6 }
    fn default_token_logo() -> String { "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz/logo.png".to_string() }
    fn default_usdc_mint() -> String { "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string() }
    fn default_rules_path() -> String { "../../proxy/rules/presets/bot-essentials.json".to_string() }
    
    let toml_config: TomlConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse platform config from {}", path))?;
    
    // Build config with env var overrides
    let mut config = PlatformConfig {
        database_url: toml_config.database.url,
        frontend_url: toml_config.frontend.url,
        frontend_cors_credentials: toml_config.frontend.cors_credentials,
        push_notifications: PushConfig {
            enabled: toml_config.push_notifications.enabled,
            public_key: if toml_config.push_notifications.public_key.is_empty() {
                None
            } else {
                Some(toml_config.push_notifications.public_key)
            },
            private_key: if toml_config.push_notifications.private_key.is_empty() {
                None
            } else {
                Some(toml_config.push_notifications.private_key)
            },
        },
        payments: PaymentConfig {
            enabled: toml_config.payments.enabled,
            token: TokenInfo {
                mint: toml_config.payments.token_mint,
                name: toml_config.payments.token_name,
                symbol: toml_config.payments.token_symbol,
                decimals: toml_config.payments.token_decimals,
                logo: toml_config.payments.token_logo,
            },
            usdc_mint: toml_config.payments.usdc_mint,
            treasury_wallet: toml_config.payments.treasury_wallet,
            pricing: PricingTiers {
                small_price: toml_config.payments.pricing.small_price,
                small_credits: toml_config.payments.pricing.small_credits,
                medium_price: toml_config.payments.pricing.medium_price,
                medium_credits: toml_config.payments.pricing.medium_credits,
                large_price: toml_config.payments.pricing.large_price,
                large_credits: toml_config.payments.pricing.large_credits,
                xlarge_price: toml_config.payments.pricing.xlarge_price,
                xlarge_credits: toml_config.payments.pricing.xlarge_credits,
            },
        },
        learning_enabled: toml_config.learning.enabled,
        rules_display_path: toml_config.display.rules_path,
    };
    
    // Apply environment variable overrides
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        log::info!("  ↳ Overriding database.url from DATABASE_URL env var");
        config.database_url = db_url;
    }
    
    if let Ok(frontend) = std::env::var("FRONTEND_URL") {
        log::info!("  ↳ Overriding frontend.url from FRONTEND_URL env var");
        config.frontend_url = frontend;
    }
    
    if let Ok(key) = std::env::var("VAPID_PUBLIC_KEY") {
        log::info!("  ↳ Overriding VAPID public key from env var");
        config.push_notifications.public_key = Some(key);
    }
    
    if let Ok(key) = std::env::var("VAPID_PRIVATE_KEY") {
        log::info!("  ↳ Overriding VAPID private key from env var");
        config.push_notifications.private_key = Some(key);
    }
    
    if let Ok(enabled) = std::env::var("PAYMENTS_ENABLED") {
        if let Ok(b) = enabled.parse::<bool>() {
            log::info!("  ↳ Overriding payments.enabled from PAYMENTS_ENABLED env var");
            config.payments.enabled = b;
        }
    }
    
    if let Ok(mint) = std::env::var("PAYMENT_TOKEN_MINT") {
        config.payments.token.mint = mint;
    }
    if let Ok(name) = std::env::var("PAYMENT_TOKEN_NAME") {
        config.payments.token.name = name;
    }
    if let Ok(symbol) = std::env::var("PAYMENT_TOKEN_SYMBOL") {
        config.payments.token.symbol = symbol;
    }
    if let Ok(decimals) = std::env::var("PAYMENT_TOKEN_DECIMALS") {
        if let Ok(d) = decimals.parse() {
            config.payments.token.decimals = d;
        }
    }
    if let Ok(logo) = std::env::var("PAYMENT_TOKEN_LOGO") {
        config.payments.token.logo = logo;
    }
    
    Ok(config)
}
