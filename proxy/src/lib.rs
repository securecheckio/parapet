// Library exports for reusable RPC proxy components

pub mod auth;
pub mod cache;
pub mod config;
pub mod escalations;
pub mod output;
pub mod rpc_handler;
pub mod server;
pub mod types;
pub mod upstream;
pub mod usage_tracker;

// Re-export rules from sol-shield library
pub use parapet_core::rules;

// Re-export for external use
pub use auth::{AuthContext, AuthProvider, AuthResult};
pub use rpc_handler::handle_rpc;
pub use server::{build_app_router, AuthMode, FeedSourceConfig, ServerConfig};
pub use parapet_core;
pub use types::AppState;
