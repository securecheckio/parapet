use anyhow::{ensure, Context, Result};
use axum::{routing::post, Json, Router};
use base64::Engine;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::{json, Value};
use parapet_proxy::{build_app_router, AuthMode, ServerConfig};
use solana_sdk::hash::Hash;
use solana_sdk::message::Message;
use solana_sdk::pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_system_interface::instruction as system_instruction;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

const MEMO_PROG: solana_sdk::pubkey::Pubkey =
    pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
const SPL_TOKEN_PROG: solana_sdk::pubkey::Pubkey =
    pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// SPL Token instruction discriminators (mirrors token_instructions.rs)
const SPL_APPROVE: u8 = 4;
const SPL_REVOKE: u8 = 5;
const SPL_FREEZE: u8 = 10;

// ── Expected HTTP outcome ────────────────────────────────────────────────────

/// What HTTP status the proxy should return for a given test case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedOutcome {
    /// Proxy forwards to upstream → 2xx.
    Pass,
    /// Rule fires with action=alert, proxy still forwards → 2xx.
    Alert,
    /// Rule fires with action=block → 403.
    Block,
}

impl ExpectedOutcome {
    pub fn is_forbidden(self) -> bool {
        self == ExpectedOutcome::Block
    }
}

// ── Test cases ───────────────────────────────────────────────────────────────

/// A named (rule, transaction, expected-outcome) triple.
/// The transaction is purpose-built to satisfy the rule's conditions.
#[derive(Clone, Debug)]
pub struct TestCase {
    /// Short label shown in the summary (e.g. "sol-transfer-pass").
    pub name: &'static str,
    /// The rule ID this case is designed to exercise.
    pub rule_id: &'static str,
    pub expected: ExpectedOutcome,
    /// Pre-encoded base64 transaction.
    pub tx_b64: String,
}

/// All built-in test cases, keyed by name.
pub struct TestCaseRegistry {
    cases: HashMap<&'static str, TestCase>,
}

impl TestCaseRegistry {
    pub fn build() -> Self {
        let payer = Keypair::new();
        let to = Keypair::new().pubkey();
        let h = Hash::default();

        let mut cases = HashMap::new();

        // ── Pass: plain SOL transfer, no rules match ─────────────────────────
        let ix = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let tx = signed_tx(&payer, &[ix], h);
        cases.insert(
            "sol-transfer-pass",
            TestCase {
                name: "sol-transfer-pass",
                rule_id: "(none)",
                expected: ExpectedOutcome::Pass,
                tx_b64: versioned_b64(&tx),
            },
        );

        // ── Alert: Memo program present ──────────────────────────────────────
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = solana_sdk::instruction::Instruction::new_with_bytes(
            MEMO_PROG, b"rpc-perf", vec![],
        );
        let tx = signed_tx(&payer, &[ix0, ix1], h);
        cases.insert(
            "memo-alert",
            TestCase {
                name: "memo-alert",
                rule_id: "rpc-perf-alert-memo",
                expected: ExpectedOutcome::Alert,
                tx_b64: versioned_b64(&tx),
            },
        );

        // ── Block: large SOL transfer (> 5 SOL) ─────────────────────────────
        // 6 SOL = 6_000_000_000 lamports — triggers the large-sol-transfer block rule.
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 6_000_000_000);
        let tx = signed_tx(&payer, &[ix0], h);
        cases.insert(
            "large-sol-transfer-block",
            TestCase {
                name: "large-sol-transfer-block",
                rule_id: "rpc-perf-block-large-sol-transfer",
                expected: ExpectedOutcome::Block,
                tx_b64: versioned_b64(&tx),
            },
        );

        // ── Block: SPL Approve with u64::MAX (unlimited delegation) ──────────
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = spl_ix(SPL_APPROVE, &u64::MAX.to_le_bytes(), 3);
        let tx = signed_tx(&payer, &[ix0, ix1], h);
        cases.insert(
            "unlimited-approve-block",
            TestCase {
                name: "unlimited-approve-block",
                rule_id: "rpc-perf-block-unlimited-approve",
                expected: ExpectedOutcome::Block,
                tx_b64: versioned_b64(&tx),
            },
        );

        // ── Block: SPL Freeze + SPL Approve combo ────────────────────────────
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = spl_ix(SPL_FREEZE, &[], 2);
        let ix2 = spl_ix(SPL_APPROVE, &1000u64.to_le_bytes(), 3);
        let tx = signed_tx(&payer, &[ix0, ix1, ix2], h);
        cases.insert(
            "freeze-combo-block",
            TestCase {
                name: "freeze-combo-block",
                rule_id: "rpc-perf-block-freeze-combo",
                expected: ExpectedOutcome::Block,
                tx_b64: versioned_b64(&tx),
            },
        );

        // ── Alert: 4-ix tx exercises full analyzer fan-out ───────────────────
        // transfer + memo + approve + revoke
        // approve+revoke = net-zero delegation (no block), memo triggers alert.
        // No Compute Budget ix — that would hit the block rule first.
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = solana_sdk::instruction::Instruction::new_with_bytes(
            MEMO_PROG, b"multi", vec![],
        );
        let ix2 = spl_ix(SPL_APPROVE, &500u64.to_le_bytes(), 3);
        let ix3 = spl_ix(SPL_REVOKE, &[], 2);
        let tx = signed_tx(&payer, &[ix0, ix1, ix2, ix3], h);
        cases.insert(
            "multi-ix-alert",
            TestCase {
                name: "multi-ix-alert",
                rule_id: "rpc-perf-alert-memo",
                expected: ExpectedOutcome::Alert,
                tx_b64: versioned_b64(&tx),
            },
        );

        // ── Pass: SPL Revoke only (explicit pass rule) ───────────────────────
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = spl_ix(SPL_REVOKE, &[], 2);
        let tx = signed_tx(&payer, &[ix0, ix1], h);
        cases.insert(
            "revoke-pass",
            TestCase {
                name: "revoke-pass",
                rule_id: "rpc-perf-pass-revoke-only",
                expected: ExpectedOutcome::Pass,
                tx_b64: versioned_b64(&tx),
            },
        );

        Self { cases }
    }

    pub fn get(&self, name: &str) -> Option<&TestCase> {
        self.cases.get(name)
    }

    /// All case names in a stable order.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "sol-transfer-pass",
            "memo-alert",
            "large-sol-transfer-block",
            "unlimited-approve-block",
            "freeze-combo-block",
            "multi-ix-alert",
            "revoke-pass",
        ]
    }
}

// ── RunConfig ────────────────────────────────────────────────────────────────

pub struct RunConfig {
    /// Which test cases to run. Each is repeated `iterations` times.
    /// Defaults to all cases if empty.
    pub cases: Vec<&'static str>,
    pub iterations: usize,
    pub warmup: usize,
    pub seed: u64,
    /// Rules JSON. Defaults to `fixtures/realistic-rules.json`.
    pub rules_path: Option<PathBuf>,
    pub blocking_threshold: u8,
    pub concurrency: usize,
}

impl RunConfig {
    fn resolve_rules_path(&self) -> Result<PathBuf> {
        let path = self.rules_path.clone().unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/realistic-rules.json")
        });
        path.canonicalize()
            .with_context(|| format!("rules_path {}", path.display()))
    }
}

// ── run ──────────────────────────────────────────────────────────────────────

pub async fn run(config: RunConfig) -> Result<()> {
    ensure!(config.iterations > 0, "iterations must be > 0");
    ensure!(config.concurrency > 0, "concurrency must be > 0");

    let registry = Arc::new(TestCaseRegistry::build());

    let case_names: Vec<&'static str> = if config.cases.is_empty() {
        TestCaseRegistry::all_names().to_vec()
    } else {
        config.cases.clone()
    };

    // Validate all requested names exist.
    for name in &case_names {
        ensure!(
            registry.get(name).is_some(),
            "unknown test case '{}'. Valid: {}",
            name,
            TestCaseRegistry::all_names().join(", ")
        );
    }

    let rules_path = config.resolve_rules_path()?;
    let rules_path_str = rules_path.to_string_lossy().into_owned();

    let mock_url = spawn_mock_upstream().await;

    let mut cfg = ServerConfig::default();
    cfg.upstream_url = mock_url;
    cfg.upstream_delay_ms = 0;
    cfg.upstream_max_concurrent = 512;
    cfg.rules_path = Some(rules_path_str);
    cfg.rule_action_override = None;
    cfg.bind_address = [127, 0, 0, 1];
    cfg.auth_mode = AuthMode::None;
    cfg.redis_url = None;
    cfg.enable_usage_tracking = false;
    cfg.wasm_analyzers_path = None;
    cfg.output_manager = None;
    cfg.default_blocking_threshold = config.blocking_threshold;
    cfg.enable_escalations = false;
    cfg.rules_feed_enabled = false;
    cfg.rules_feed_sources = None;

    let app = build_app_router(cfg).await.context("build_app_router")?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind proxy")?;
    let proxy_addr = listener.local_addr().context("local_addr")?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("rpc-perf: proxy server stopped: {e}");
        }
    });

    let proxy_url = format!("http://{}", proxy_addr);
    wait_health(&proxy_url)
        .await
        .context("proxy /health did not become ready")?;

    let client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .context("reqwest client")?;

    // Build schedule: each case repeated `iterations` times, shuffled.
    let schedule: Vec<&'static str> = {
        let mut s: Vec<&'static str> = case_names
            .iter()
            .flat_map(|&n| std::iter::repeat(n).take(config.iterations))
            .collect();
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
        s.shuffle(&mut rng);
        s
    };

    // Warmup (not measured).
    for i in 0..config.warmup {
        let name = schedule[i % schedule.len()];
        let tc = registry.get(name).unwrap();
        send_one(&client, &proxy_url, i as u64, &tc.tx_b64).await?;
    }

    // Measured run.
    let mut samples: HashMap<&'static str, Vec<f64>> = HashMap::new();
    for &name in &case_names {
        samples.insert(name, Vec::new());
    }
    let mut outcome_mismatches: Vec<String> = Vec::new();

    let base_id = config.warmup as u64;

    if config.concurrency <= 1 {
        for (k, &name) in schedule.iter().enumerate() {
            let tc = registry.get(name).unwrap();
            let id = base_id + k as u64;
            let t0 = Instant::now();
            let status = send_one(&client, &proxy_url, id, &tc.tx_b64).await?;
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            samples.get_mut(name).unwrap().push(ms);
            check_outcome(tc, status, &mut outcome_mismatches);
        }
    } else {
        let registry = registry.clone();
        let mut next_id = base_id;
        for chunk in schedule.chunks(config.concurrency) {
            let mut js = JoinSet::new();
            for &name in chunk {
                let id = next_id;
                next_id += 1;
                let tc = registry.get(name).unwrap().clone();
                let client = client.clone();
                let url = proxy_url.clone();
                js.spawn(async move {
                    let t0 = Instant::now();
                    let status = send_one(&client, &url, id, &tc.tx_b64).await?;
                    Ok::<_, anyhow::Error>((tc, t0.elapsed().as_secs_f64() * 1000.0, status))
                });
            }
            while let Some(joined) = js.join_next().await {
                let (tc, ms, status) = joined??;
                samples.get_mut(tc.name).unwrap().push(ms);
                check_outcome(&tc, status, &mut outcome_mismatches);
            }
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    println!("rpc-perf summary");
    println!("  proxy:       {}", proxy_url);
    println!("  upstream:    mock (localhost), upstream_delay_ms=0");
    println!("  rules:       {}", rules_path.display());
    println!(
        "  iterations:  {} per case (warmup {}), concurrency {}, total {}",
        config.iterations,
        config.warmup,
        config.concurrency,
        schedule.len()
    );
    println!("  threshold:   {}", config.blocking_threshold);
    println!();
    println!(
        "  {:<26} {:<8} {:<10} {:<10} {:<10}",
        "case", "expect", "p50ms", "p95ms", "p99ms"
    );
    println!("  {}", "-".repeat(66));

    let mut any_mismatch = false;
    for &name in &case_names {
        let tc = registry.get(name).unwrap();
        let v = samples.get(name).unwrap();
        if v.is_empty() {
            continue;
        }
        let mut sorted = v.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mismatch_count = outcome_mismatches
            .iter()
            .filter(|m| m.starts_with(name))
            .count();
        let flag = if mismatch_count > 0 {
            any_mismatch = true;
            format!(" !! {mismatch_count} outcome mismatch(es)")
        } else {
            String::new()
        };
        println!(
            "  {:<26} {:<8} {:>8.3}  {:>8.3}  {:>8.3}{}",
            name,
            format!("{:?}", tc.expected).to_lowercase(),
            percentile(&sorted, 50.0),
            percentile(&sorted, 95.0),
            percentile(&sorted, 99.0),
            flag,
        );
    }

    if any_mismatch {
        println!();
        println!("  OUTCOME MISMATCHES (rule/tx mismatch — check rules file):");
        for m in &outcome_mismatches {
            println!("    {}", m);
        }
    }

    Ok(())
}

fn check_outcome(
    tc: &TestCase,
    status: reqwest::StatusCode,
    mismatches: &mut Vec<String>,
) {
    let got_forbidden = status == reqwest::StatusCode::FORBIDDEN;
    let expected_forbidden = tc.expected.is_forbidden();
    if got_forbidden != expected_forbidden {
        mismatches.push(format!(
            "{}: expected {:?} (forbidden={}), got HTTP {}",
            tc.name, tc.expected, expected_forbidden, status
        ));
    }
}

// ── smoke helper (used by tests/smoke.rs) ────────────────────────────────────

/// One `sendRawTransaction` (pass-only tx) against proxy + minimal rules; asserts HTTP success.
pub async fn smoke_send_raw_pass() -> Result<()> {
    let rules_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/minimal-rules.json");
    let rules_path = rules_path
        .canonicalize()
        .with_context(|| format!("minimal rules {:?}", rules_path))?;

    let mock_url = spawn_mock_upstream().await;

    let mut cfg = ServerConfig::default();
    cfg.upstream_url = mock_url;
    cfg.upstream_delay_ms = 0;
    cfg.upstream_max_concurrent = 64;
    cfg.rules_path = Some(rules_path.to_string_lossy().into_owned());
    cfg.bind_address = [127, 0, 0, 1];
    cfg.auth_mode = AuthMode::None;
    cfg.redis_url = None;
    cfg.enable_usage_tracking = false;
    cfg.wasm_analyzers_path = None;
    cfg.output_manager = None;
    cfg.enable_escalations = false;
    cfg.rules_feed_enabled = false;
    cfg.rules_feed_sources = None;

    let app = build_app_router(cfg).await.context("build_app_router")?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind proxy")?;
    let proxy_addr = listener.local_addr().context("local_addr")?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("rpc-perf smoke: proxy stopped: {e}");
        }
    });

    let proxy_url = format!("http://{}", proxy_addr);
    wait_health(&proxy_url)
        .await
        .context("smoke: proxy /health did not become ready")?;

    let registry = TestCaseRegistry::build();
    let tc = registry.get("sol-transfer-pass").unwrap();
    let client = reqwest::Client::new();
    let status = send_one(&client, &proxy_url, 1, &tc.tx_b64).await?;
    ensure!(
        status.is_success(),
        "expected HTTP 2xx for pass + minimal rules, got {status}"
    );
    Ok(())
}

// ── internals ────────────────────────────────────────────────────────────────

async fn wait_health(proxy_base: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("health client");
    let url = format!("{}/health", proxy_base.trim_end_matches('/'));
    for _ in 0..100 {
        if client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err(anyhow::anyhow!("timeout waiting for GET {}", url))
}

pub async fn send_one(
    client: &reqwest::Client,
    proxy_url: &str,
    id: u64,
    tx_b64: &str,
) -> Result<reqwest::StatusCode> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "sendRawTransaction",
        "params": [tx_b64, { "encoding": "base64", "skipPreflight": true }]
    });
    let res = client
        .post(proxy_url)
        .json(&body)
        .send()
        .await
        .context("POST proxy")?;
    let status = res.status();
    res.bytes().await.context("read RPC response body")?;
    Ok(status)
}

async fn spawn_mock_upstream() -> String {
    let app = Router::new().route("/", post(mock_upstream_handler));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock bind");
    let addr = listener.local_addr().expect("mock addr");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("rpc-perf: mock upstream stopped: {e}");
        }
    });
    format!("http://{}", addr)
}

async fn mock_upstream_handler(Json(req): Json<Value>) -> Json<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": "rpcPerfStubSig111111111111111111111111111111111111111111111111"
    }))
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len().saturating_sub(1)) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Build and sign a transaction from a slice of instructions.
fn signed_tx(payer: &Keypair, instructions: &[solana_sdk::instruction::Instruction], h: Hash) -> Transaction {
    let msg = Message::new(instructions, Some(&payer.pubkey()));
    let mut tx = Transaction::new_unsigned(msg);
    tx.sign(&[payer], h);
    tx
}

fn versioned_b64(tx: &Transaction) -> String {
    let vt = VersionedTransaction::from(tx.clone());
    let bytes = bincode::serialize(&vt).expect("serialize tx");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Build a raw SPL Token instruction with `n_accounts` dummy account metas.
fn spl_ix(discriminator: u8, extra_data: &[u8], n_accounts: usize) -> solana_sdk::instruction::Instruction {
    use solana_sdk::instruction::{AccountMeta, Instruction};
    let accounts: Vec<AccountMeta> = (0..n_accounts)
        .map(|_| AccountMeta::new(solana_sdk::pubkey::Pubkey::new_unique(), false))
        .collect();
    let mut data = vec![discriminator];
    data.extend_from_slice(extra_data);
    Instruction::new_with_bytes(SPL_TOKEN_PROG, &data, accounts)
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cases_build() {
        let reg = TestCaseRegistry::build();
        for &name in TestCaseRegistry::all_names() {
            assert!(reg.get(name).is_some(), "missing case: {name}");
        }
    }

    #[test]
    fn schedule_repeats_each_case() {
        let names = TestCaseRegistry::all_names();
        let n = 10usize;
        let schedule: Vec<&str> = names
            .iter()
            .flat_map(|&name| std::iter::repeat(name).take(n))
            .collect();
        for &name in names {
            assert_eq!(
                schedule.iter().filter(|&&s| s == name).count(),
                n,
                "case {name} should appear {n} times"
            );
        }
    }
}
