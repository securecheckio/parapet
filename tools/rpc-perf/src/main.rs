//! CLI for `rpc_perf::harness`. Run `rpc-perf --help`.

use anyhow::Result;
use clap::Parser;
use rpc_perf::harness::{self, RunConfig, TestCaseRegistry};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rpc-perf")]
#[command(
    about = "Measure proxy + rule-engine latency per test case (mock upstream, synthetic sendRawTransaction).",
    long_about = "\
Each test case is a (rule, transaction, expected-outcome) triple. \
The transaction is purpose-built to satisfy the rule's conditions. \
All cases run `--iterations` times each, shuffled together. \
Default rules file: fixtures/realistic-rules.json.\n\
\n\
Built-in cases:\n\
  sol-transfer-pass         plain SOL transfer (1 lamport) → pass\n\
  memo-alert                transfer + Memo ix              → alert\n\
  large-sol-transfer-block  SOL transfer > 5 SOL            → block\n\
  unlimited-approve-block   SPL Approve u64::MAX            → block\n\
  freeze-combo-block        SPL Freeze + Approve            → block\n\
  multi-ix-alert            4-ix fan-out (memo present)     → alert\n\
  revoke-pass               SPL Revoke only                 → pass"
)]
struct Cli {
    /// Test cases to run (comma-separated). Defaults to all cases.
    /// Example: --cases unlimited-approve-block,freeze-combo-block
    #[arg(long, value_delimiter = ',')]
    cases: Vec<String>,

    /// Number of times each case is repeated (all cases shuffled together).
    #[arg(long, default_value_t = 100)]
    iterations: usize,

    /// Warmup iterations (not measured).
    #[arg(long, default_value_t = 20)]
    warmup: usize,

    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Rules JSON file. Defaults to fixtures/realistic-rules.json.
    #[arg(long)]
    rules_path: Option<PathBuf>,

    /// Rule-engine blocking threshold (0-100).
    #[arg(long, default_value_t = 70)]
    blocking_threshold: u8,

    /// Parallel in-flight JSON-RPC requests (1 = sequential).
    #[arg(long, default_value_t = 1)]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate case names early.
    let valid = TestCaseRegistry::all_names();
    let cases: Vec<&'static str> = cli
        .cases
        .iter()
        .map(|s| {
            valid
                .iter()
                .copied()
                .find(|&v| v == s.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown case '{}'. Valid: {}",
                        s,
                        valid.join(", ")
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;

    let config = RunConfig {
        cases,
        iterations: cli.iterations,
        warmup: cli.warmup,
        seed: cli.seed,
        rules_path: cli.rules_path,
        blocking_threshold: cli.blocking_threshold,
        concurrency: cli.concurrency,
    };
    harness::run(config).await
}
