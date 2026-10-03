use crate::config::SentinelConfig;
use crate::plugin::Plugin;
use crate::transpiler::SentinelTranspiler;
use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

pub struct SentinelWatcher {
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<notify::Event>>,
    app_roots: Vec<PathBuf>,
    output_path: PathBuf,
    shared_paths: Vec<PathBuf>,
}

impl SentinelWatcher {
    /// Initializes the watcher on all configured folders.
    pub fn new(config: &SentinelConfig) -> Result<Self> {
        let (tx, rx) = mpsc::channel(100);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            Config::default(),
        )?;

        let mut app_roots = Vec::new();
        for folder in config.folder_paths() {
            let canonical = folder.canonicalize().unwrap_or_else(|_| folder.clone());
            watcher.watch(&canonical, RecursiveMode::Recursive)?;
            app_roots.push(canonical);
        }

        Ok(Self {
            watcher,
            receiver: rx,
            app_roots,
            output_path: config.output_path(),
            shared_paths: config.shared_type_paths(),
        })
    }

    /// Check if a path is a real .rb file (not a temp file from sed, editors, etc.)
    fn is_watchable_rb(path: &Path) -> bool {
        let ext_ok = path.extension().is_some_and(|ext| ext == "rb");
        if !ext_ok {
            return false;
        }
        match path.file_name().and_then(|f| f.to_str()) {
            Some(name) => !name.starts_with('.') && !name.contains('~'),
            None => false,
        }
    }

    /// The main event loop that processes file changes and triggers transpilation
    pub async fn run(mut self) {
        let mut transpiler = SentinelTranspiler::new();
        transpiler.set_shared_paths(self.shared_paths.clone());
        for root in &self.app_roots {
            println!("🚀 Sentinel standing guard over {:?}...", root);
        }

        while let Some(res) = self.receiver.recv().await {
            // Collect paths from this event
            let mut changed_paths = HashSet::new();
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() {
                    for path in event.paths {
                        if Self::is_watchable_rb(&path) {
                            changed_paths.insert(path);
                        }
                    }
                }
            }

            // Debounce: wait briefly, then drain and collect all pending events
            sleep(Duration::from_millis(50)).await;
            while let Ok(res) = self.receiver.try_recv() {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        for path in event.paths {
                            if Self::is_watchable_rb(&path) {
                                changed_paths.insert(path);
                            }
                        }
                    }
                }
            }

            // Process all unique changed .rb files
            for path in &changed_paths {
                self.handle_change(&mut transpiler, path).await;
            }
        }
    }

    /// Find which app_root contains this path
    fn app_root_for(&self, path: &Path) -> Option<&PathBuf> {
        self.app_roots.iter().find(|root| path.starts_with(root))
    }

    async fn handle_change(&self, transpiler: &mut SentinelTranspiler, path: &Path) {
        println!(
            "🔍 Change detected: {:?}",
            path.file_name().unwrap_or_default()
        );

        let result = transpiler.transpile_file(path);
        for w in transpiler.take_warnings() {
            eprintln!("⚠️  [warn] {}:{}: {}", path.display(), w.line, w.message);
        }
        match result {
            Ok(rbs_content) => {
                // 1. RUN PLUGINS (The new linter system)
                for plugin in Plugin::ALL {
                    let issues = plugin.check(&rbs_content);
                    if !issues.is_empty() {
                        eprintln!(
                            "⚠️  [{}] issues in {:?}:",
                            plugin.name(),
                            path.file_name().unwrap_or_default()
                        );
                        for (method, msg) in issues {
                            eprintln!("   - Method `{}`: {}", method, msg);
                        }
                    }
                }

                let target_path = self.derive_sig_path(path);

                // 2. Ensure directory exists
                if let Some(parent) = target_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                // 3. Write file
                if let Err(e) = std::fs::write(&target_path, &rbs_content) {
                    eprintln!("❌ Failed to write RBS: {}", e);
                } else {
                    println!("✅ RBS Synced -> {:?}", target_path);
                }
            }
            Err(e) => eprintln!("❌ Transpiler error: {}", e),
        }
    }

    fn derive_sig_path(&self, rb_path: &Path) -> PathBuf {
        let mut p = self.output_path.clone();
        if let Some(app_root) = self.app_root_for(rb_path) {
            if let Ok(relative) = rb_path.strip_prefix(app_root) {
                p.push(relative);
            } else if let Some(name) = rb_path.file_name() {
                p.push(name);
            }
        } else if let Some(name) = rb_path.file_name() {
            p.push(name);
        }
        p.set_extension("rbs");
        p
    }
}
