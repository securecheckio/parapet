// Integration tests
// Tests are organized in the integration/ subdirectory

#[cfg(feature = "wasm-analyzers")]
mod wasm_tests {
    include!("integration/wasm_integration.rs");
}

mod third_party_tests {
    include!("integration/third_party_analyzers.rs");
}
