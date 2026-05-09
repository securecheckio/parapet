use anyhow::{ensure, Context, Result};
use axum::{routing::post, Json, Router};
use base64::Engine;
use bs58;
use parapet_rpc_proxy::{build_app_router, AuthMode, ServerConfig};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::{json, Value};
use solana_address_lookup_table_interface::state::{AddressLookupTable, LookupTableMeta};
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::compiled_instruction::CompiledInstruction;
use solana_sdk::message::v0::{Message as MessageV0, MessageAddressTableLookup};
use solana_sdk::message::{Message, MessageHeader, VersionedMessage};
use solana_sdk::pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;
use std::borrow::Cow;
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

const SPL_APPROVE: u8 = 4;
const SPL_REVOKE: u8 = 5;
const SPL_FREEZE: u8 = 10;

// ── Expected HTTP outcome ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedOutcome {
    Pass,
    Alert,
    Block,
}

impl ExpectedOutcome {
    pub fn is_forbidden(self) -> bool {
        self == ExpectedOutcome::Block
    }
}

/// Send path (`sendRawTransaction`) vs simulate (`simulateTransaction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarnessMode {
    Send,
    Simulate,
}

// ── Test cases ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: &'static str,
    pub rule_id: &'static str,
    pub expected: ExpectedOutcome,
    pub tx_b64: String,
}

pub struct TestCaseRegistry {
    cases: HashMap<&'static str, TestCase>,
    pub alt_cache_preseed: Vec<(String, Vec<u8>)>,
}

impl TestCaseRegistry {
    pub fn build() -> Self {
        let payer = Keypair::new();
        let to = Keypair::new().pubkey();
        let h = Hash::default();
        let mut cases = HashMap::new();
        let mut alt_cache_preseed: Vec<(String, Vec<u8>)> = Vec::new();

        // ── Pass: legacy-in-versioned wrapper ────────────────────────────────
        let ix = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let tx = signed_tx(&payer, &[ix.clone()], h);
        cases.insert(
            "sol-transfer-pass-legacy",
            TestCase {
                name: "sol-transfer-pass-legacy",
                rule_id: "(none)",
                expected: ExpectedOutcome::Pass,
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

        // ── Pass: v0 message, empty address-table lookups ────────────────────
        let vt_empty =
            signed_v0_empty(&payer, &[ix], h).expect("build v0 empty lookups SOL transfer");
        cases.insert(
            "sol-transfer-pass-v0-empty",
            TestCase {
                name: "sol-transfer-pass-v0-empty",
                rule_id: "(none)",
                expected: ExpectedOutcome::Pass,
                tx_b64: versioned_b64_tx(&vt_empty),
            },
        );

        // ── Pass: v0 + ALT (recipient loaded from lookup table; cache pre-seeded)
        let alt_kp = Keypair::new();
        let alt_pk = alt_kp.pubkey();
        let alt_table = AddressLookupTable {
            meta: LookupTableMeta::default(),
            addresses: Cow::Owned(vec![to]),
        };
        let alt_bytes = alt_table
            .serialize_for_tests()
            .unwrap_or_else(|e| panic!("ALT serialize_for_tests: {:?}", e));
        alt_cache_preseed.push((alt_pk.to_string(), alt_bytes));
        let vt_alt = signed_v0_sol_transfer_with_alt(&payer, to, alt_pk, 1, h)
            .expect("build v0 ALT SOL transfer");
        cases.insert(
            "sol-transfer-pass-v0-alt",
            TestCase {
                name: "sol-transfer-pass-v0-alt",
                rule_id: "(none)",
                expected: ExpectedOutcome::Pass,
                tx_b64: versioned_b64_tx(&vt_alt),
            },
        );

        // ── Alert: Memo program present ──────────────────────────────────────
        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 =
            solana_sdk::instruction::Instruction::new_with_bytes(MEMO_PROG, b"rpc-perf", vec![]);
        let tx = signed_tx(&payer, &[ix0, ix1], h);
        cases.insert(
            "memo-alert",
            TestCase {
                name: "memo-alert",
                rule_id: "rpc-perf-alert-memo",
                expected: ExpectedOutcome::Alert,
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 6_000_000_000);
        let tx = signed_tx(&payer, &[ix0], h);
        cases.insert(
            "large-sol-transfer-block",
            TestCase {
                name: "large-sol-transfer-block",
                rule_id: "rpc-perf-block-large-sol-transfer",
                expected: ExpectedOutcome::Block,
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = spl_ix(SPL_APPROVE, &u64::MAX.to_le_bytes(), 3);
        let tx = signed_tx(&payer, &[ix0, ix1], h);
        cases.insert(
            "unlimited-approve-block",
            TestCase {
                name: "unlimited-approve-block",
                rule_id: "rpc-perf-block-unlimited-approve",
                expected: ExpectedOutcome::Block,
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

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
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = solana_sdk::instruction::Instruction::new_with_bytes(MEMO_PROG, b"multi", vec![]);
        let ix2 = spl_ix(SPL_APPROVE, &500u64.to_le_bytes(), 3);
        let ix3 = spl_ix(SPL_REVOKE, &[], 2);
        let tx = signed_tx(&payer, &[ix0, ix1, ix2, ix3], h);
        cases.insert(
            "multi-ix-alert",
            TestCase {
                name: "multi-ix-alert",
                rule_id: "rpc-perf-alert-memo",
                expected: ExpectedOutcome::Alert,
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

        let ix0 = system_instruction::transfer(&payer.pubkey(), &to, 1);
        let ix1 = spl_ix(SPL_REVOKE, &[], 2);
        let tx = signed_tx(&payer, &[ix0, ix1], h);
        cases.insert(
            "revoke-pass",
            TestCase {
                name: "revoke-pass",
                rule_id: "rpc-perf-pass-revoke-only",
                expected: ExpectedOutcome::Pass,
                tx_b64: versioned_b64_legacy(&tx),
            },
        );

        Self {
            cases,
            alt_cache_preseed,
        }
    }

    pub fn get(&self, name: &str) -> Option<&TestCase> {
        self.cases.get(name)
    }

    pub fn all_names() -> &'static [&'static str] {
        &[
            "sol-transfer-pass-legacy",
            "sol-transfer-pass-v0-empty",
            "sol-transfer-pass-v0-alt",
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
    pub harness_mode: HarnessMode,
    pub cases: Vec<&'static str>,
    pub iterations: usize,
    pub warmup: usize,
    pub seed: u64,
    pub rules_path: Option<PathBuf>,
    pub blocking_threshold: u8,
    pub concurrency: usize,
}

impl RunConfig {
    fn resolve_rules_path(&self) -> Result<PathBuf> {
        let path = self.rules_path.clone().unwrap_or_else(|| {
            let rel = match self.harness_mode {
                HarnessMode::Send => "fixtures/baseline-rules.json",
                HarnessMode::Simulate => "fixtures/simulation-rules.json",
            };
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
        });
        path.canonicalize()
            .with_context(|| format!("rules_path {}", path.display()))
    }
}

// ── run ──────────────────────────────────────────────────────────────────────

pub async fn run(config: RunConfig) -> Result<()> {
    ensure!(config.iterations > 0, "iterations must be > 0");
    ensure!(config.concurrency > 0, "concurrency must be > 0");

    let registry_inner = TestCaseRegistry::build();
    let needs_alt_seed = config.cases.is_empty()
        || config
            .cases
            .iter()
            .any(|&n| n == "sol-transfer-pass-v0-alt");
    let alt_preseed = if needs_alt_seed {
        registry_inner.alt_cache_preseed.clone()
    } else {
        Vec::new()
    };
    let registry = Arc::new(registry_inner);

    let case_names: Vec<&'static str> = if config.cases.is_empty() {
        TestCaseRegistry::all_names().to_vec()
    } else {
        config.cases.clone()
    };

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
    cfg.alt_cache_preseed = alt_preseed;

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

    let schedule: Vec<&'static str> = {
        let mut s: Vec<&'static str> = case_names
            .iter()
            .flat_map(|&n| std::iter::repeat(n).take(config.iterations))
            .collect();
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
        s.shuffle(&mut rng);
        s
    };

    let mode = config.harness_mode;

    for i in 0..config.warmup {
        let name = schedule[i % schedule.len()];
        let tc = registry.get(name).unwrap();
        harness_rpc(mode, &client, &proxy_url, i as u64, &tc.tx_b64).await?;
    }

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
            let (status, body) = harness_rpc(mode, &client, &proxy_url, id, &tc.tx_b64).await?;
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            samples.get_mut(name).unwrap().push(ms);
            check_outcome_dispatch(
                mode,
                tc,
                status,
                body.as_ref(),
                config.blocking_threshold,
                &mut outcome_mismatches,
            );
        }
    } else {
        let registry = registry.clone();
        let mut next_id = base_id;
        let blocking_threshold = config.blocking_threshold;
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
                    let (status, body) = harness_rpc(mode, &client, &url, id, &tc.tx_b64).await?;
                    Ok::<_, anyhow::Error>((tc, t0.elapsed().as_secs_f64() * 1000.0, status, body))
                });
            }
            while let Some(joined) = js.join_next().await {
                let (tc, ms, status, body) = joined??;
                samples.get_mut(tc.name).unwrap().push(ms);
                check_outcome_dispatch(
                    mode,
                    &tc,
                    status,
                    body.as_ref(),
                    blocking_threshold,
                    &mut outcome_mismatches,
                );
            }
        }
    }

    println!("rpc-perf summary");
    println!("  mode:        {:?}", mode);
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
        anyhow::bail!("rpc-perf finished with outcome mismatches");
    }

    Ok(())
}

fn check_outcome_dispatch(
    mode: HarnessMode,
    tc: &TestCase,
    status: reqwest::StatusCode,
    body: &[u8],
    blocking_threshold: u8,
    mismatches: &mut Vec<String>,
) {
    match mode {
        HarnessMode::Send => check_send_outcome(tc, status, mismatches),
        HarnessMode::Simulate => {
            check_simulate_outcome(tc, status, body, blocking_threshold, mismatches)
        }
    }
}

fn check_send_outcome(tc: &TestCase, status: reqwest::StatusCode, mismatches: &mut Vec<String>) {
    let got_forbidden = status == reqwest::StatusCode::FORBIDDEN;
    let expected_forbidden = tc.expected.is_forbidden();
    if got_forbidden != expected_forbidden {
        mismatches.push(format!(
            "{}: expected {:?} (forbidden={}), got HTTP {}",
            tc.name, tc.expected, expected_forbidden, status
        ));
    }
}

fn check_simulate_outcome(
    tc: &TestCase,
    status: reqwest::StatusCode,
    body: &[u8],
    blocking_threshold: u8,
    mismatches: &mut Vec<String>,
) {
    if !status.is_success() {
        mismatches.push(format!(
            "{}: simulate expected HTTP 2xx, got {}",
            tc.name, status
        ));
        return;
    }
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            mismatches.push(format!("{}: invalid JSON response: {}", tc.name, e));
            return;
        }
    };
    if v.get("error").is_some() {
        mismatches.push(format!("{}: JSON-RPC error in simulate response", tc.name));
        return;
    }
    let decision = v
        .get("result")
        .and_then(|r| r.get("parapet"))
        .and_then(|p| p.get("decision"))
        .and_then(|d| d.as_str());
    if !simulate_parapet_decision_ok(tc.expected, blocking_threshold, decision) {
        mismatches.push(format!(
            "{}: unexpected parapet decision {:?} (threshold {}, expected {:?})",
            tc.name, decision, blocking_threshold, tc.expected
        ));
    }
}

/// Default rule weight when metadata omits `weight` matches [`RuleEngine`] simulation loop (`20`).
const DEFAULT_RULE_WEIGHT_SIM: u8 = 20;

fn simulate_parapet_decision_ok(
    expected: ExpectedOutcome,
    blocking_threshold: u8,
    decision: Option<&str>,
) -> bool {
    match expected {
        ExpectedOutcome::Pass => decision == Some("safe"),
        ExpectedOutcome::Alert => matches!(decision, Some("alert") | Some("would_block")),
        ExpectedOutcome::Block => {
            // Structural blocks are downgraded to alerts in simulation; a single default-weight
            // rule yields total_risk 20. Parapet labels `would_block` when total_risk >= threshold.
            let single_rule_would_threshold =
                decision == Some("would_block") && blocking_threshold <= DEFAULT_RULE_WEIGHT_SIM;
            let single_rule_alert_band =
                decision == Some("alert") && blocking_threshold > DEFAULT_RULE_WEIGHT_SIM;
            single_rule_would_threshold || single_rule_alert_band
        }
    }
}

// ── smoke helper (used by tests/smoke.rs) ────────────────────────────────────

pub async fn smoke_send_raw_pass() -> Result<()> {
    let rules_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/empty-rules.json");
    let rules_path = rules_path
        .canonicalize()
        .with_context(|| format!("empty rules {:?}", rules_path))?;

    let mock_url = spawn_mock_upstream().await;

    let cfg = ServerConfig {
        upstream_url: mock_url,
        upstream_delay_ms: 0,
        upstream_max_concurrent: 64,
        rules_path: Some(rules_path.to_string_lossy().into_owned()),
        bind_address: [127, 0, 0, 1],
        auth_mode: AuthMode::None,
        redis_url: None,
        enable_usage_tracking: false,
        wasm_analyzers_path: None,
        output_manager: None,
        enable_escalations: false,
        rules_feed_enabled: false,
        rules_feed_sources: None,
        ..Default::default()
    };

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
    let tc = registry.get("sol-transfer-pass-legacy").unwrap();
    let client = reqwest::Client::new();
    let (status, _) = harness_rpc(HarnessMode::Send, &client, &proxy_url, 1, &tc.tx_b64).await?;
    ensure!(
        status.is_success(),
        "expected HTTP 2xx for pass + empty rules, got {status}"
    );
    Ok(())
}

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

pub async fn harness_rpc(
    mode: HarnessMode,
    client: &reqwest::Client,
    proxy_url: &str,
    id: u64,
    tx_b64: &str,
) -> Result<(reqwest::StatusCode, Vec<u8>)> {
    let (method, params) = match mode {
        HarnessMode::Send => (
            "sendRawTransaction",
            vec![
                json!(tx_b64),
                json!({ "encoding": "base64", "skipPreflight": true }),
            ],
        ),
        HarnessMode::Simulate => (
            "simulateTransaction",
            vec![
                json!(tx_b64),
                json!({ "encoding": "base64", "replaceRecentBlockhash": true }),
            ],
        ),
    };
    let body = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    let res = client
        .post(proxy_url)
        .json(&body)
        .send()
        .await
        .context("POST proxy")?;
    let status = res.status();
    let bytes = res
        .bytes()
        .await
        .context("read RPC response body")?
        .to_vec();
    Ok((status, bytes))
}

async fn spawn_mock_upstream() -> String {
    let app = Router::new().route("/", post(mock_upstream_handler));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("mock bind");
    let addr = listener.local_addr().expect("mock addr");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("rpc-perf: mock upstream stopped: {e}");
        }
    });
    format!("http://{}", addr)
}

fn simulation_logs_for_request(req: &Value) -> Vec<Value> {
    let tx_param = req
        .get("params")
        .and_then(|p| p.as_array())
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(Value::Null);
    if tx_contains_memo_program(&tx_param) {
        vec![json!("Program log: warning rpc-perf memo simulation")]
    } else {
        vec![]
    }
}

fn tx_contains_memo_program(tx_data: &Value) -> bool {
    let Some(s) = tx_data.as_str() else {
        return false;
    };
    let tx_bytes = match bs58::decode(s).into_vec() {
        Ok(v) => v,
        Err(_) => base64::engine::general_purpose::STANDARD
            .decode(s)
            .unwrap_or_default(),
    };
    if tx_bytes.is_empty() {
        return false;
    }
    let Ok(vt) = bincode::deserialize::<VersionedTransaction>(&tx_bytes) else {
        return false;
    };
    let keys = vt.message.static_account_keys();
    for ix in vt.message.instructions() {
        let Some(pid) = keys.get(ix.program_id_index as usize) else {
            continue;
        };
        if *pid == MEMO_PROG {
            return true;
        }
    }
    false
}

async fn mock_upstream_handler(Json(req): Json<Value>) -> Json<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    match method {
        "simulateTransaction" => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "context": { "slot": 100 },
                "value": {
                    "accounts": Value::Null,
                    "err": Value::Null,
                    "fee": 5000,
                    "innerInstructions": Value::Null,
                    "loadedAccountsDataSize": 0,
                    "logs": simulation_logs_for_request(&req),
                    "postBalances": Value::Array(vec![]),
                    "postTokenBalances": Value::Null,
                    "preBalances": Value::Array(vec![]),
                    "preTokenBalances": Value::Null,
                    "replacementBlockhash": Value::Null,
                    "returnData": Value::Null,
                    "unitsConsumed": 0
                }
            }
        })),
        _ => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": "rpcPerfStubSig111111111111111111111111111111111111111111111111"
        })),
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len().saturating_sub(1)) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn signed_tx(
    payer: &Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
    h: Hash,
) -> Transaction {
    let msg = Message::new(instructions, Some(&payer.pubkey()));
    let mut tx = Transaction::new_unsigned(msg);
    tx.sign(&[payer], h);
    tx
}

fn versioned_b64_legacy(tx: &Transaction) -> String {
    versioned_b64_tx(&VersionedTransaction::from(tx.clone()))
}

fn versioned_b64_tx(tx: &VersionedTransaction) -> String {
    let bytes = bincode::serialize(tx).expect("serialize tx");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn signed_v0_empty(
    payer: &Keypair,
    instructions: &[Instruction],
    _recent_blockhash: Hash,
) -> Result<VersionedTransaction> {
    let legacy = Message::new(instructions, Some(&payer.pubkey()));
    let v0 = MessageV0 {
        header: legacy.header,
        account_keys: legacy.account_keys.clone(),
        recent_blockhash: legacy.recent_blockhash,
        instructions: legacy.instructions.clone(),
        address_table_lookups: Vec::new(),
    };
    VersionedTransaction::try_new(VersionedMessage::V0(v0), &[payer])
        .map_err(|e| anyhow::anyhow!("v0 empty sign failed: {}", e))
}

fn signed_v0_sol_transfer_with_alt(
    payer: &Keypair,
    to: solana_sdk::pubkey::Pubkey,
    alt_table_address: solana_sdk::pubkey::Pubkey,
    lamports: u64,
    recent_blockhash: Hash,
) -> Result<VersionedTransaction> {
    let payer_pk = payer.pubkey();
    let sys = system_program::id();
    let transfer_ix = system_instruction::transfer(&payer_pk, &to, lamports);
    let header = MessageHeader {
        num_required_signatures: 1,
        num_readonly_signed_accounts: 0,
        num_readonly_unsigned_accounts: 1,
    };
    let account_keys = vec![payer_pk, sys];
    let compiled = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2],
        data: transfer_ix.data,
    };
    let v0 = MessageV0 {
        header,
        account_keys,
        recent_blockhash,
        instructions: vec![compiled],
        address_table_lookups: vec![MessageAddressTableLookup {
            account_key: alt_table_address,
            writable_indexes: vec![0],
            readonly_indexes: vec![],
        }],
    };
    VersionedTransaction::try_new(VersionedMessage::V0(v0), &[payer])
        .map_err(|e| anyhow::anyhow!("v0 ALT sign failed: {}", e))
}

fn spl_ix(
    discriminator: u8,
    extra_data: &[u8],
    n_accounts: usize,
) -> solana_sdk::instruction::Instruction {
    use solana_sdk::instruction::{AccountMeta, Instruction};
    let accounts: Vec<AccountMeta> = (0..n_accounts)
        .map(|_| AccountMeta::new(solana_sdk::pubkey::Pubkey::new_unique(), false))
        .collect();
    let mut data = vec![discriminator];
    data.extend_from_slice(extra_data);
    Instruction::new_with_bytes(SPL_TOKEN_PROG, &data, accounts)
}

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
