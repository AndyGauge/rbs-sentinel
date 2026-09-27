mod check;
mod config;
mod init;
mod lsp;
mod plugin;
mod transpiler;
mod watcher;

use anyhow::Result;
use config::SentinelConfig;
use watcher::SentinelWatcher;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("watch");

    match command {
        "init" => {
            let config = SentinelConfig::ensure_exists()?;
            std::fs::create_dir_all(config.output_path())?;
            let shared = config.shared_type_paths();
            for folder in config.folder_paths() {
                init::run(&folder, &config.output_path(), &shared, config.emit_superclasses);
            }
        }
        "check" => {
            let config = SentinelConfig::load()?;
            let shared = config.shared_type_paths();
            let mut all_ok = true;
            for folder in config.folder_paths() {
                if !check::run(&folder, &config.output_path(), &shared, config.emit_superclasses) {
                    all_ok = false;
                }
            }
            if !all_ok {
                std::process::exit(1);
            }
        }
        "watch" => {
            let config = SentinelConfig::ensure_exists()?;
            std::fs::create_dir_all(config.output_path())?;
            let shared = config.shared_type_paths();
            for folder in config.folder_paths() {
                init::run(&folder, &config.output_path(), &shared, config.emit_superclasses);
            }
            let watcher = SentinelWatcher::new(&config)?.with_plugins();
            watcher.run().await;
        }
        "add" => {
            let folder = args.get(2).map(|s| s.as_str()).unwrap_or_else(|| {
                eprintln!("Usage: sentinel add <folder>");
                std::process::exit(1);
            });
            let mut config = SentinelConfig::ensure_exists()?;
            if config.add_folder(folder) {
                config.save()?;
                println!("Added '{}' — watching: {:?}", folder, config.folders);
            } else {
                println!("'{}' is already in the watch list.", folder);
            }
        }
        "remove" => {
            let folder = args.get(2).map(|s| s.as_str()).unwrap_or_else(|| {
                eprintln!("Usage: sentinel remove <folder>");
                std::process::exit(1);
            });
            let mut config = SentinelConfig::load()?;
            if config.remove_folder(folder) {
                config.save()?;
                println!("Removed '{}' — watching: {:?}", folder, config.folders);
            } else {
                println!("'{}' is not in the watch list.", folder);
            }
        }
        "list" => {
            let config = SentinelConfig::load()?;
            println!("Watched folders:");
            for folder in &config.folders {
                println!("  {}", folder);
            }
            println!("Output: {}", config.output);
        }
        "lsp" => {
            lsp::run().await;
        }
        other => {
            eprintln!("Unknown command: {}", other);
            eprintln!("Usage: sentinel [init|watch|check|add|remove|list|lsp]");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  init           Generate RBS files for all watched folders");
            eprintln!("  watch          Watch for changes and regenerate (default)");
            eprintln!("  check          Verify signatures are up to date (exit 1 if stale)");
            eprintln!("  add <folder>   Add a folder to the watch list");
            eprintln!("  remove <folder> Remove a folder from the watch list");
            eprintln!("  list           Show watched folders and output path");
            eprintln!("  lsp            Run as a Language Server (stdio); publishes plugin");
            eprintln!("                 diagnostics and syncs sig/generated on save");
            std::process::exit(1);
        }
    }

    Ok(())
}
