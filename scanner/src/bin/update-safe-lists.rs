/// CLI tool to update known-safe programs and owners from remote feeds
use anyhow::Result;
use clap::Parser;
use parapet_core::rules::analyzers::FeedUpdater;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "update-safe-lists")]
#[command(about = "Update known-safe programs and owners from remote feeds", long_about = None)]
struct Args {
    /// Directory containing config files (default: proxy/config)
    #[arg(short, long)]
    config_dir: Option<PathBuf>,

    /// Custom feed URL for programs (overrides config file)
    #[arg(long)]
    programs_feed_url: Option<String>,

    /// Custom feed URL for owners (overrides config file)
    #[arg(long)]
    owners_feed_url: Option<String>,

    /// Update only programs list
    #[arg(long)]
    programs_only: bool,

    /// Update only owners list
    #[arg(long)]
    owners_only: bool,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "info" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Determine config directory
    let config_dir = args.config_dir.unwrap_or_else(|| {
        // Try to find proxy/config relative to current directory
        let candidates = vec![
            PathBuf::from("proxy/config"),
            PathBuf::from("../proxy/config"),
            PathBuf::from("../../proxy/config"),
            PathBuf::from("parapet/proxy/config"),
        ];

        candidates
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from("proxy/config"))
    });

    println!("📂 Config directory: {}", config_dir.display());
    println!();

    // Create updater
    let updater = FeedUpdater::new();

    // Update based on flags
    if args.programs_only {
        // Update only programs
        let path = config_dir.join("known-safe-programs.json");
        println!("🔄 Updating safe programs list...");
        match updater.update_programs(&path, args.programs_feed_url.as_deref()) {
            Ok(true) => println!("✅ Safe programs list updated successfully"),
            Ok(false) => println!("ℹ️  Safe programs list is already up to date"),
            Err(e) => {
                eprintln!("❌ Failed to update safe programs: {}", e);
                std::process::exit(1);
            }
        }
    } else if args.owners_only {
        // Update only owners
        let path = config_dir.join("known-safe-owners.json");
        println!("🔄 Updating safe owners list...");
        match updater.update_owners(&path, args.owners_feed_url.as_deref()) {
            Ok(true) => println!("✅ Safe owners list updated successfully"),
            Ok(false) => println!("ℹ️  Safe owners list is already up to date"),
            Err(e) => {
                eprintln!("❌ Failed to update safe owners: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Update both
        println!("🔄 Updating safe programs and owners lists...");
        match updater.update_all(&config_dir) {
            Ok((programs_updated, owners_updated)) => {
                println!();
                if programs_updated {
                    println!("✅ Safe programs list updated");
                } else {
                    println!("ℹ️  Safe programs list already up to date");
                }

                if owners_updated {
                    println!("✅ Safe owners list updated");
                } else {
                    println!("ℹ️  Safe owners list already up to date");
                }

                if !programs_updated && !owners_updated {
                    println!("\n✨ All lists are up to date");
                } else {
                    println!("\n✨ Update complete");
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to update lists: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
