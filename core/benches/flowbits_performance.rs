use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use parapet_core::rules::flowbits::FlowbitStateManager;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

fn bench_per_wallet_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("per_wallet_operations");

    // Benchmark set operation
    group.bench_function("set", |b| {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();
        b.iter(|| {
            manager.set(
                black_box(&wallet),
                black_box("test_flowbit"),
                Some(Duration::from_secs(3600)),
            );
        });
    });

    // Benchmark increment operation
    group.bench_function("increment", |b| {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();
        b.iter(|| {
            manager.increment(
                black_box(&wallet),
                black_box("test_counter"),
                Some(Duration::from_secs(3600)),
            );
        });
    });

    // Benchmark is_set check
    group.bench_function("is_set", |b| {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();
        manager.set(&wallet, "test_flowbit", Some(Duration::from_secs(3600)));
        b.iter(|| {
            black_box(manager.is_set(black_box(&wallet), black_box("test_flowbit")));
        });
    });

    // Benchmark get_counter
    group.bench_function("get_counter", |b| {
        let mut manager = FlowbitStateManager::new(None);
        let wallet = Pubkey::new_unique();
        manager.increment(&wallet, "test_counter", Some(Duration::from_secs(3600)));
        b.iter(|| {
            black_box(manager.get_counter(black_box(&wallet), black_box("test_counter")));
        });
    });

    group.finish();
}

fn bench_global_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("global_operations");

    // Benchmark global set
    group.bench_function("set_global", |b| {
        let mut manager = FlowbitStateManager::new(None);
        b.iter(|| {
            manager.set_global(black_box("test_flowbit"), Some(Duration::from_secs(3600)));
        });
    });

    // Benchmark global increment
    group.bench_function("increment_global", |b| {
        let mut manager = FlowbitStateManager::new(None);
        b.iter(|| {
            manager.increment_global(black_box("test_counter"), Some(Duration::from_secs(3600)));
        });
    });

    // Benchmark global is_set check
    group.bench_function("is_set_global", |b| {
        let mut manager = FlowbitStateManager::new(None);
        manager.set_global("test_flowbit", Some(Duration::from_secs(3600)));
        b.iter(|| {
            black_box(manager.is_set_global(black_box("test_flowbit")));
        });
    });

    // Benchmark global get_counter
    group.bench_function("get_counter_global", |b| {
        let mut manager = FlowbitStateManager::new(None);
        manager.increment_global("test_counter", Some(Duration::from_secs(3600)));
        b.iter(|| {
            black_box(manager.get_counter_global(black_box("test_counter")));
        });
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    // Benchmark with increasing number of wallets
    for num_wallets in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("wallets", num_wallets),
            num_wallets,
            |b, &num_wallets| {
                let mut manager = FlowbitStateManager::new(None);
                let wallets: Vec<Pubkey> = (0..num_wallets).map(|_| Pubkey::new_unique()).collect();

                // Pre-populate with flowbits
                for wallet in &wallets {
                    manager.increment(wallet, "transaction_count", Some(Duration::from_secs(3600)));
                }

                b.iter(|| {
                    let wallet = &wallets[num_wallets / 2];
                    black_box(
                        manager.get_counter(black_box(wallet), black_box("transaction_count")),
                    );
                });
            },
        );
    }

    // Benchmark with increasing number of global keys
    for num_keys in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("global_keys", num_keys),
            num_keys,
            |b, &num_keys| {
                let mut manager = FlowbitStateManager::new(None);

                // Pre-populate with global flowbits
                for i in 0..num_keys {
                    manager.increment_global(
                        &format!("recipient_{}", i),
                        Some(Duration::from_secs(3600)),
                    );
                }

                b.iter(|| {
                    let key = format!("recipient_{}", num_keys / 2);
                    black_box(manager.get_counter_global(black_box(&key)));
                });
            },
        );
    }

    group.finish();
}

fn bench_variable_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("variable_interpolation");

    // Benchmark simple interpolation (no variables)
    group.bench_function("no_variables", |b| {
        let template = "simple_flowbit_name";
        b.iter(|| {
            black_box(template.contains('{'));
        });
    });

    // Benchmark single variable interpolation
    group.bench_function("single_variable", |b| {
        let template = "transfers_to:{recipient}";
        let recipient = "7xKHnfHvPfVvFVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV";
        b.iter(|| {
            let result = template.replace("{recipient}", recipient);
            black_box(result);
        });
    });

    // Benchmark multiple variable interpolation
    group.bench_function("multiple_variables", |b| {
        let template = "transfer_{mint}_to_{recipient}";
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let recipient = "7xKHnfHvPfVvFVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV";
        b.iter(|| {
            let result = template
                .replace("{mint}", mint)
                .replace("{recipient}", recipient);
            black_box(result);
        });
    });

    group.finish();
}

fn bench_realistic_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_scenarios");

    // AI Agent: 10 transactions in 10 minutes
    group.bench_function("ai_agent_velocity", |b| {
        let mut manager = FlowbitStateManager::new(Some(1));
        let wallet = Pubkey::new_unique();

        b.iter(|| {
            // Increment counter
            manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));
            // Check threshold
            let count = manager.get_counter(&wallet, "transaction_count");
            black_box(count >= 10);
        });
    });

    // Enterprise: Lateral movement detection (3 wallets, same recipient)
    group.bench_function("enterprise_lateral_movement", |b| {
        let mut manager = FlowbitStateManager::new(None);
        let recipient = "AttackerAddress111111111111111111111111111";

        b.iter(|| {
            // Increment global counter
            manager.increment_global(
                &format!("suspicious_recipient:{}", recipient),
                Some(Duration::from_secs(3600)),
            );
            // Check threshold
            let count = manager.get_counter_global(&format!("suspicious_recipient:{}", recipient));
            black_box(count > 2);
        });
    });

    // AI Agent: Gradual exfiltration (per-recipient tracking)
    group.bench_function("ai_agent_exfiltration", |b| {
        let mut manager = FlowbitStateManager::new(Some(1));
        let wallet = Pubkey::new_unique();
        let recipient = "AttackerAddress111111111111111111111111111";

        b.iter(|| {
            // Increment per-recipient counter
            manager.increment(
                &wallet,
                &format!("transfers_to:{}", recipient),
                Some(Duration::from_secs(86400)),
            );
            // Check threshold
            let count = manager.get_counter(&wallet, &format!("transfers_to:{}", recipient));
            black_box(count > 3);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_per_wallet_operations,
    bench_global_operations,
    bench_scaling,
    bench_variable_interpolation,
    bench_realistic_scenarios
);
criterion_main!(benches);
