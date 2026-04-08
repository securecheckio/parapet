#[cfg(feature = "wasm-analyzers")]
use anyhow::{anyhow, Result};
#[cfg(feature = "wasm-analyzers")]
use serde_json::Value;
#[cfg(feature = "wasm-analyzers")]
use solana_sdk::transaction::Transaction;
#[cfg(feature = "wasm-analyzers")]
use std::collections::HashMap;
#[cfg(feature = "wasm-analyzers")]
use std::path::PathBuf;
#[cfg(feature = "wasm-analyzers")]
use std::sync::Arc;

#[cfg(feature = "wasm-analyzers")]
use wasmtime::*;

#[cfg(feature = "wasm-analyzers")]
use super::analyzer::TransactionAnalyzer;

#[cfg(feature = "wasm-analyzers")]
struct WasmConfig {
    config: HashMap<String, String>,
}

/// WASM Analyzer loader and runtime
#[cfg(feature = "wasm-analyzers")]
pub struct WasmAnalyzer {
    name: String,
    fields: Vec<String>,
    engine: Engine,
    module: Module,
    estimated_latency: u64,
    config: HashMap<String, String>,
}

#[cfg(feature = "wasm-analyzers")]
impl WasmAnalyzer {
    /// Load a WASM analyzer from a file with explicit config values
    pub fn from_file(path: &PathBuf, user_config: HashMap<String, String>) -> Result<Self> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid WASM file name"))?
            .to_string();

        log::info!("📦 Loading WASM analyzer: {} from {:?}", name, path);

        // Create WASM engine with security constraints
        let mut wasm_config = Config::new();
        wasm_config.wasm_multi_memory(false);
        wasm_config.wasm_threads(false);
        wasm_config.wasm_reference_types(true); // Required by Rust WASM
        wasm_config.wasm_simd(false);
        wasm_config.wasm_relaxed_simd(false);
        wasm_config.wasm_bulk_memory(true); // Required by Rust WASM
        wasm_config.consume_fuel(true); // Enable fuel metering for timeout protection

        let engine = Engine::new(&wasm_config)?;
        let module = Module::from_file(&engine, path)?;

        // Log imports
        log::info!("WASM module imports:");
        for import in module.imports() {
            log::info!(
                "  - {}::{} (type: {:?})",
                import.module(),
                import.name(),
                import.ty()
            );
        }

        // Validate required exports
        let mut has_analyze = false;
        let mut has_fields = false;
        let mut has_estimated_latency = false;

        log::info!("WASM module exports:");
        for export in module.exports() {
            log::info!("  - {} (type: {:?})", export.name(), export.ty());
            match export.name() {
                "analyze" => has_analyze = true,
                "get_fields" => has_fields = true,
                "get_estimated_latency_ms" => has_estimated_latency = true,
                _ => {}
            }
        }

        if !has_analyze {
            return Err(anyhow!("WASM module missing required 'analyze' function"));
        }
        if !has_fields {
            return Err(anyhow!(
                "WASM module missing required 'get_fields' function"
            ));
        }

        // Extract fields from WASM module
        let fields = Self::extract_fields(&engine, &module)?;
        let estimated_latency = if has_estimated_latency {
            Self::extract_estimated_latency(&engine, &module)?
        } else {
            10 // Default 10ms for WASM analyzers
        };

        log::info!(
            "✅ Loaded WASM analyzer '{}' with {} fields",
            name,
            fields.len()
        );

        Ok(Self {
            name,
            fields,
            engine,
            module,
            estimated_latency,
            config: user_config,
        })
    }

    fn extract_fields(engine: &Engine, module: &Module) -> Result<Vec<String>> {
        let mut store = Store::new(engine, ());
        store.set_fuel(10_000_000)?; // Higher fuel limit for metadata extraction

        // Check if module needs get_config import
        let needs_config = module
            .imports()
            .any(|i| i.module() == "env" && i.name() == "get_config");

        let instance = if needs_config {
            let mut linker: Linker<()> = Linker::new(engine);
            linker.func_wrap(
                "env",
                "get_config",
                |caller: Caller<'_, ()>, key_ptr: i32, key_len: i32, value_ptr_out: i32| -> i32 {
                    let _ = (caller, key_ptr, key_len, value_ptr_out);
                    0 // Return empty for metadata extraction
                },
            )?;
            linker.instantiate(&mut store, module)?
        } else {
            Instance::new(&mut store, module, &[])?
        };

        let get_fields = instance.get_typed_func::<(), i64>(&mut store, "get_fields")?;

        let packed = get_fields.call(&mut store, ())?;

        // Unpack i64 into ptr and len
        let ptr = (packed >> 32) as i32;
        let len = (packed & 0xFFFFFFFF) as i32;

        // Read JSON string from WASM memory
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("WASM module has no memory export"))?;

        let data = memory.data(&store);
        let json_bytes = &data[ptr as usize..(ptr + len) as usize];
        let json_str = std::str::from_utf8(json_bytes)?;

        let fields: Vec<String> = serde_json::from_str(json_str)?;
        Ok(fields)
    }

    fn extract_estimated_latency(engine: &Engine, module: &Module) -> Result<u64> {
        let mut store = Store::new(engine, ());
        store.set_fuel(10_000_000)?; // Higher fuel limit for metadata extraction

        // Check if module needs get_config import
        let needs_config = module
            .imports()
            .any(|i| i.module() == "env" && i.name() == "get_config");

        let instance = if needs_config {
            let mut linker: Linker<()> = Linker::new(engine);
            linker.func_wrap(
                "env",
                "get_config",
                |caller: Caller<'_, ()>, key_ptr: i32, key_len: i32, value_ptr_out: i32| -> i32 {
                    let _ = (caller, key_ptr, key_len, value_ptr_out);
                    0 // Return empty for metadata extraction
                },
            )?;
            linker.instantiate(&mut store, module)?
        } else {
            Instance::new(&mut store, module, &[])?
        };

        let get_latency =
            instance.get_typed_func::<(), i64>(&mut store, "get_estimated_latency_ms")?;

        let latency = get_latency.call(&mut store, ())?;
        Ok(latency as u64)
    }

    fn call_analyze(&self, tx_bytes: &[u8], config: HashMap<String, String>) -> Result<String> {
        let mut store = Store::new(&self.engine, WasmConfig { config });
        store.set_fuel(1_000_000)?;

        let mut linker: Linker<WasmConfig> = Linker::new(&self.engine);

        // Add host function for config access
        linker.func_wrap(
            "env",
            "get_config",
            |mut caller: Caller<'_, WasmConfig>,
             key_ptr: i32,
             key_len: i32,
             value_ptr_out: i32|
             -> i32 {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(mem) => mem,
                    None => return 0,
                };

                let data = memory.data(&caller);
                if key_ptr < 0 || key_len < 0 || (key_ptr as usize + key_len as usize) > data.len()
                {
                    return 0;
                }

                let key_bytes = &data[key_ptr as usize..(key_ptr + key_len) as usize];
                let key = match std::str::from_utf8(key_bytes) {
                    Ok(s) => s,
                    Err(_) => return 0,
                };

                // Get value from store context
                let value = caller.data().config.get(key).cloned().unwrap_or_default();
                let value_bytes = value.as_bytes();

                if !value_bytes.is_empty() && value_ptr_out >= 0 {
                    if memory
                        .write(&mut caller, value_ptr_out as usize, value_bytes)
                        .is_err()
                    {
                        return 0;
                    }
                }

                value_bytes.len() as i32
            },
        )?;

        let instance = linker.instantiate(&mut store, &self.module)?;

        // Get memory and allocate space for transaction
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("WASM module has no memory export"))?;

        // Get allocate function
        let allocate = instance.get_typed_func::<i32, i32>(&mut store, "allocate")?;

        // Allocate memory in WASM
        let ptr = allocate.call(&mut store, tx_bytes.len() as i32)?;

        // Copy transaction bytes to WASM memory
        memory.write(&mut store, ptr as usize, tx_bytes)?;

        // Call analyze function (returns packed i64 with ptr in high 32 bits, len in low 32 bits)
        let analyze = instance.get_typed_func::<(i32, i32), i64>(&mut store, "analyze")?;

        let packed = analyze.call(&mut store, (ptr, tx_bytes.len() as i32))?;
        let result_ptr = (packed >> 32) as i32;
        let result_len = (packed & 0xFFFFFFFF) as i32;

        // Read result from WASM memory
        let data = memory.data(&store);
        let result_bytes = &data[result_ptr as usize..(result_ptr + result_len) as usize];
        let result_str = std::str::from_utf8(result_bytes)?.to_string();

        Ok(result_str)
    }
}

#[cfg(feature = "wasm-analyzers")]
#[async_trait::async_trait]
impl TransactionAnalyzer for WasmAnalyzer {
    fn name(&self) -> &str {
        &self.name
    }

    fn fields(&self) -> Vec<String> {
        self.fields.clone()
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        // Serialize transaction to bytes
        let tx_bytes = bincode::serialize(tx)?;

        // Use config provided at load time (user explicitly chose what to expose)
        let result_json = self.call_analyze(&tx_bytes, self.config.clone())?;

        // Parse result
        let fields: HashMap<String, Value> = serde_json::from_str(&result_json)?;

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        self.estimated_latency
    }
}

/// Load all WASM analyzers from a directory
#[cfg(feature = "wasm-analyzers")]
pub fn load_wasm_analyzers_from_dir(
    dir_path: &str,
    config: HashMap<String, String>,
) -> Result<Vec<Arc<dyn TransactionAnalyzer>>> {
    let path = PathBuf::from(dir_path);

    if !path.exists() {
        log::info!("📁 WASM analyzers directory not found: {:?}", path);
        return Ok(Vec::new());
    }

    if !path.is_dir() {
        return Err(anyhow!(
            "WASM analyzers path is not a directory: {:?}",
            path
        ));
    }

    let mut analyzers: Vec<Arc<dyn TransactionAnalyzer>> = Vec::new();

    log::info!("🔍 Scanning for WASM analyzers in {:?}", path);

    for entry in std::fs::read_dir(&path)? {
        let entry = entry?;
        let file_path = entry.path();

        // Only process .wasm files
        if file_path.extension().and_then(|s| s.to_str()) != Some("wasm") {
            continue;
        }

        match WasmAnalyzer::from_file(&file_path, config.clone()) {
            Ok(analyzer) => {
                log::info!("✅ Loaded WASM analyzer: {}", analyzer.name());
                analyzers.push(Arc::new(analyzer));
            }
            Err(e) => {
                log::error!(
                    "❌ Failed to load WASM analyzer from {:?}: {}",
                    file_path,
                    e
                );
            }
        }
    }

    log::info!("📦 Loaded {} WASM analyzer(s)", analyzers.len());

    Ok(analyzers)
}

/// No-op implementations when wasm-analyzers feature is disabled
#[cfg(not(feature = "wasm-analyzers"))]
pub fn load_wasm_analyzers_from_dir(
    dir_path: &str,
    config: std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<std::sync::Arc<dyn super::analyzer::TransactionAnalyzer>>> {
    let _ = (dir_path, config);
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_wasm_feature_flag() {
        // This test verifies that the module compiles with and without the feature
        #[cfg(feature = "wasm-analyzers")]
        {
            println!("WASM analyzers feature is enabled");
        }

        #[cfg(not(feature = "wasm-analyzers"))]
        {
            println!("WASM analyzers feature is disabled");
        }
    }
}
