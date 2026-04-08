// Mock WASM Analyzer for Testing
use std::collections::HashMap;

// Host function to get config values (provided by parapet)
#[link(wasm_import_module = "env")]
extern "C" {
    fn get_config(key_ptr: *const u8, key_len: i32, value_ptr: *mut u8) -> i32;
}

// Helper to safely call get_config
fn get_config_value(key: &str) -> Option<String> {
    let mut buffer = vec![0u8; 512]; // Max config value size
    let value_len = unsafe {
        get_config(key.as_ptr(), key.len() as i32, buffer.as_mut_ptr())
    };
    
    if value_len > 0 {
        buffer.truncate(value_len as usize);
        String::from_utf8(buffer).ok()
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn allocate(size: i32) -> *mut u8 {
    let mut buf = Vec::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn get_fields() -> i64 {
    let fields = vec![
        "mock_field_1".to_string(),
        "mock_field_2".to_string(),
        "mock_risk_score".to_string(),
    ];
    
    let json = serde_json::to_string(&fields).unwrap();
    let bytes = json.into_bytes();
    let len = bytes.len() as i32;
    let ptr = bytes.as_ptr() as i32;
    std::mem::forget(bytes);
    
    // Pack ptr and len into i64
    ((ptr as i64) << 32) | (len as i64 & 0xFFFFFFFF)
}

#[no_mangle]
pub extern "C" fn get_estimated_latency_ms() -> i64 {
    5
}

#[no_mangle]
pub extern "C" fn analyze(tx_ptr: i32, tx_len: i32) -> i64 {
    let tx_bytes = unsafe {
        std::slice::from_raw_parts(tx_ptr as *const u8, tx_len as usize)
    };
    let _ = tx_bytes;
    
    // Try to access config to verify it was passed
    let api_key = get_config_value("HELIUS_API_KEY");
    let has_config = api_key.is_some();
    
    let mut fields = HashMap::new();
    fields.insert("mock_field_1".to_string(), serde_json::json!("test_value"));
    fields.insert("mock_field_2".to_string(), serde_json::json!(42));
    fields.insert("mock_risk_score".to_string(), serde_json::json!(25));
    fields.insert("mock_has_config".to_string(), serde_json::json!(has_config));
    
    let json = serde_json::to_string(&fields).unwrap();
    let bytes = json.into_bytes();
    let len = bytes.len() as i32;
    let ptr = bytes.as_ptr() as i32;
    std::mem::forget(bytes);
    
    // Pack ptr and len into i64
    ((ptr as i64) << 32) | (len as i64 & 0xFFFFFFFF)
}
