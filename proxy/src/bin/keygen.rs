//! Simple API key generator for Parapet
//! 
//! Usage:
//!   cargo run --bin keygen
//!   cargo run --bin keygen -- --count 5
//!   cargo run --bin keygen -- --prefix sk_prod

use rand::Rng;
use std::env;

fn generate_api_key(prefix: &str) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const KEY_LENGTH: usize = 48; // 48 chars = 286 bits of entropy
    
    let mut rng = rand::thread_rng();
    let random_part: String = (0..KEY_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    
    format!("{}_{}", prefix, random_part)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut count = 1;
    let mut prefix = "sc_test".to_string();
    
    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--count" | "-c" => {
                if i + 1 < args.len() {
                    count = args[i + 1].parse().unwrap_or(1);
                    i += 1;
                }
            }
            "--prefix" | "-p" => {
                if i + 1 < args.len() {
                    prefix = args[i + 1].clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Parapet API Key Generator");
                println!();
                println!("Usage:");
                println!("  keygen [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -c, --count <N>       Generate N keys (default: 1)");
                println!("  -p, --prefix <PREFIX> Key prefix (default: sc_test)");
                println!("  -h, --help            Show this help");
                println!();
                println!("Examples:");
                println!("  cargo run --bin keygen");
                println!("  cargo run --bin keygen -- --count 5");
                println!("  cargo run --bin keygen -- --prefix sc_prod --count 3");
                println!();
                println!("After generating keys, add them to your .env file:");
                println!("  API_KEYS=\"sc_test_abc:user1|sc_prod_xyz:user2\"");
                return;
            }
            _ => {}
        }
        i += 1;
    }
    
    println!("🔑 Generating {} API key(s) with prefix '{}'", count, prefix);
    println!();
    
    let mut keys = Vec::new();
    for i in 1..=count {
        let key = generate_api_key(&prefix);
        keys.push(key.clone());
        if count == 1 {
            println!("{}", key);
        } else {
            println!("Key {}: {}", i, key);
        }
    }
    
    println!();
    println!("📝 Add to your .env file:");
    if count == 1 {
        println!("API_KEYS=\"{}:your_user_id\"", keys[0]);
    } else {
        print!("API_KEYS=\"");
        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                print!("|");
            }
            print!("{}:user{}", key, i + 1);
        }
        println!("\"");
    }
}
