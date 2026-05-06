//! Debounced filesystem watch for `[security] rules_path` so local rule edits apply without SIGHUP.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parapet_core::rules::RuleEngine;
use tokio::sync::RwLock;

const DEBOUNCE_MS: u64 = 400;

fn rules_watch_disabled() -> bool {
    matches!(
        std::env::var("PARAPET_RULES_WATCH")
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Ok("0" | "false" | "no" | "off")
    )
}

fn event_affects_rules_path(event: &Event, watch_mode: &WatchMode) -> bool {
    if event.paths.is_empty() {
        return false;
    }

    match watch_mode {
        WatchMode::Directory(dir) => {
            let dir_canon = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.clone());
            for p in &event.paths {
                if p.extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                let p_canon = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                if let Ok(rest) = p_canon.strip_prefix(&dir_canon) {
                    if rest.as_os_str().is_empty() {
                        continue;
                    }
                    if rest.components().count() == 1 {
                        return true;
                    }
                }
            }
            false
        }
        WatchMode::File(file) => {
            for p in &event.paths {
                if p == file {
                    return true;
                }
                let same = (|| {
                    let a = std::fs::canonicalize(p).ok()?;
                    let b = std::fs::canonicalize(file).ok()?;
                    Some(a == b)
                })();
                if same == Some(true) {
                    return true;
                }
            }
            false
        }
    }
}

fn should_consider_event(kind: &EventKind) -> bool {
    !kind.is_access() && !kind.is_other()
}

#[derive(Clone, Debug)]
enum WatchMode {
    File(PathBuf),
    Directory(PathBuf),
}

/// Reload rules from disk into the shared engine (same operation as SIGHUP's rules step).
/// Preserves feed rules by extracting them before reload and merging them back after.
pub async fn reload_rules_from_disk(
    rule_engine: &RwLock<RuleEngine>,
    path: &str,
) -> anyhow::Result<()> {
    let mut engine = rule_engine.write().await;

    // Extract current feed rules (non-local rules)
    let feed_rules: Vec<parapet_core::rules::RuleDefinition> = engine
        .rules()
        .iter()
        .filter(|r| !r.id.starts_with("custom-") && !r.id.starts_with("local-"))
        .cloned()
        .collect();

    let feed_count = feed_rules.len();

    // Load local rules from file (this replaces all rules temporarily)
    engine.load_rules_from_file(path)?;

    // If we had feed rules, merge them back in
    if feed_count > 0 {
        log::info!("🔄 Restoring {} feed rules after local reload", feed_count);
        engine.merge_rules(feed_rules)?;
    }

    Ok(())
}

fn resolve_watch_target(rules_path: &Path) -> Option<(PathBuf, WatchMode)> {
    if rules_path.is_dir() {
        let dir = std::fs::canonicalize(rules_path).ok()?;
        return Some((dir.clone(), WatchMode::Directory(dir)));
    }

    let file = rules_path.to_path_buf();
    let parent = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let watch_root = std::fs::canonicalize(&parent).unwrap_or(parent);

    Some((watch_root, WatchMode::File(file)))
}

async fn debounced_reload_loop(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<notify::Result<Event>>,
    rule_engine: std::sync::Arc<RwLock<RuleEngine>>,
    rules_path_str: String,
    watch_mode: WatchMode,
) {
    while let Some(res) = rx.recv().await {
        let event = match res {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Rules file watcher error: {}", e);
                continue;
            }
        };

        if !should_consider_event(&event.kind) {
            continue;
        }

        if !event_affects_rules_path(&event, &watch_mode) {
            continue;
        }

        tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
        while rx.try_recv().is_ok() {}

        log::info!(
            "📋 Rules file change detected — reloading {}",
            rules_path_str
        );
        match reload_rules_from_disk(rule_engine.as_ref(), &rules_path_str).await {
            Ok(()) => log::info!(
                "✅ Rules auto-reloaded ({} enabled rules)",
                rule_engine.read().await.enabled_rule_count()
            ),
            Err(e) => log::error!(
                "❌ Rules auto-reload failed (keeping previous rules): {}",
                e
            ),
        }
    }
}

/// Spawn a background task that watches `rules_path` and reloads the rule engine after debounced saves.
///
/// Disabled when `PARAPET_RULES_WATCH` is `0`, `false`, `no`, or `off` (case-insensitive).
/// No-op when `rules_path` is missing, empty, or the watch root does not exist.
pub fn spawn_rules_file_watcher(
    rules_path: std::sync::Arc<Option<String>>,
    rule_engine: std::sync::Arc<RwLock<RuleEngine>>,
) {
    if rules_watch_disabled() {
        log::info!("📭 Rules file auto-watch disabled (PARAPET_RULES_WATCH)");
        return;
    }

    let Some(rules_path_str) = rules_path.as_deref().filter(|s| !s.is_empty()) else {
        return;
    };

    let rules_pb = PathBuf::from(rules_path_str);
    let Some((watch_root, watch_mode)) = resolve_watch_target(&rules_pb) else {
        log::warn!(
            "📭 Rules file watch skipped — could not resolve watch path for {}",
            rules_pb.display()
        );
        return;
    };

    if !watch_root.exists() {
        log::warn!(
            "📭 Rules file watch skipped — path does not exist yet: {}",
            watch_root.display()
        );
        return;
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<notify::Result<Event>>();

    let watch_root_display = watch_root.display().to_string();
    let rules_path_owned = rules_path_str.to_string();
    let rules_path_for_log = rules_path_owned.clone();
    let watch_mode_for_task = watch_mode.clone();

    // `RecommendedWatcher` must stay alive for the process lifetime; keep it on a blocking thread.
    std::thread::spawn(move || {
        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(err) => {
                log::error!("❌ Failed to create rules file watcher: {}", err);
                return;
            }
        };

        if let Err(err) = watcher.watch(&watch_root, RecursiveMode::NonRecursive) {
            log::error!(
                "❌ Failed to watch {} for rules reload: {}",
                watch_root_display,
                err
            );
            return;
        }

        log::info!(
            "👀 Watching {} for changes to {} (debounce {}ms; set PARAPET_RULES_WATCH=0 to disable)",
            watch_root_display,
            rules_path_for_log,
            DEBOUNCE_MS
        );

        loop {
            std::thread::park();
        }
    });

    tokio::spawn(debounced_reload_loop(
        rx,
        rule_engine,
        rules_path_owned,
        watch_mode_for_task,
    ));
}
