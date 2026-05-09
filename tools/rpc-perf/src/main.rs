//! CLI for `rpc_perf::harness`. Run `rpc-perf send --help` / `rpc-perf simulate --help`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use rpc_perf::harness::{self, HarnessMode, RunConfig, TestCaseRegistry};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rpc-perf")]
#[command(
    about = "Measure proxy + rule-engine latency per test case (mock upstream).",
    long_about = "\
Subcommands:\n\
  send      Synthetic sendRawTransaction (403 on block).\n\
  simulate  Synthetic simulateTransaction (HTTP 200; parapet.decision in JSON).\n\
\n\
Each test case is a (rule, transaction, expected-outcome) triple. \
All selected cases run `--iterations` times each, shuffled together.\n\
Default rules: send → fixtures/baseline-rules.json; simulate → fixtures/simulation-rules.json.\n\
\n\
Built-in cases include legacy / v0 / v0+ALT SOL-transfer passes plus memo, SPL, and fan-out scenarios.\n\
See --help on a subcommand for the full case list."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// sendRawTransaction path (HTTP 403 when a block rule fires).
    Send(PerfArgs),
    /// simulateTransaction path (check result.parapet.decision).
    Simulate(PerfArgs),
}

#[derive(Parser, Debug)]
struct PerfArgs {
    /// Test cases to run (comma-separated). Defaults to all cases.
    #[arg(long, value_delimiter = ',')]
    cases: Vec<String>,

    #[arg(long, default_value_t = 100)]
    iterations: usize,

    #[arg(long, default_value_t = 20)]
    warmup: usize,

    #[arg(long, default_value_t = 42)]
    seed: u64,

    #[arg(long)]
    rules_path: Option<PathBuf>,

    #[arg(long, default_value_t = 70)]
    blocking_threshold: u8,

    #[arg(long, default_value_t = 1)]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let (mode, args) = match cli.command {
        Commands::Send(a) => (HarnessMode::Send, a),
        Commands::Simulate(a) => (HarnessMode::Simulate, a),
    };

    let valid = TestCaseRegistry::all_names();
    let cases: Vec<&'static str> = args
        .cases
        .iter()
        .map(|s| {
            valid
                .iter()
                .copied()
                .find(|&v| v == s.as_str())
                .ok_or_else(|| anyhow::anyhow!("unknown case '{}'. Valid: {}", s, valid.join(", ")))
        })
        .collect::<Result<Vec<_>>>()?;

    let config = RunConfig {
        harness_mode: mode,
        cases,
        iterations: args.iterations,
        warmup: args.warmup,
        seed: args.seed,
        rules_path: args.rules_path,
        blocking_threshold: args.blocking_threshold,
        concurrency: args.concurrency,
    };
    harness::run(config).await
}
