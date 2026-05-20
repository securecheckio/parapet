//! Parapet MCP - Shared MCP Tool Implementations
//!
//! This library contains the canonical implementations of all MCP tools.
//! Used by both:
//! - STDIO MCP server (this crate's binary)
//! - HTTP MCP server (parapet-api)
//!
//! **All MCP tool logic lives here** - this is the single source of truth.

pub mod helius_tools;
pub mod scan_coverage;
pub mod scan_types;
pub mod tools;

// Re-export commonly used items
pub use scan_coverage::*;
pub use scan_types::*;
pub use tools::*;
