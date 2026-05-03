//! Re-exports shared Solana JSON-RPC upstream types from [`parapet_upstream`].

pub use parapet_upstream::{
    build_upstream_stack, build_upstream_stack_with_strategy, parse_upstream_urls_list,
    CircuitState, FailoverUpstreamProvider, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    SmartUpstreamProvider, UpstreamClient, UpstreamHttpSettings, UpstreamProvider,
};

/// Backwards-compatible alias used by tests and older docs.
pub type UpstreamConfig = UpstreamHttpSettings;
