use crate::plugin::Plugin;
use crate::transpiler::SentinelTranspiler;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

pub fn run(app_path: &Path, output_path: &Path, shared_paths: &[PathBuf], emit_superclasses: bool) {
    let start = Instant::now();
    let app_root = app_path
        .canonicalize()
        .unwrap_or_else(|_| app_path.to_path_buf());

    let files: Vec<PathBuf> = WalkDir::new(&app_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().is_some_and(|ext| ext == "rb")
                && path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .is_some_and(|name| !name.starts_with('.') && !name.contains('~'))
        })
        .map(|e| e.into_path())
        .collect();

    let total = files.len();
    println!("Found {} Ruby files in {:?}", total, app_path);

    let success = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    files.par_iter().for_each(|path| {
        let mut transpiler = SentinelTranspiler::new();
        transpiler.set_emit_superclasses(emit_superclasses);
        transpiler.set_shared_paths(shared_paths.to_vec());

        let result = transpiler.transpile_file(path);
        for w in transpiler.take_warnings() {
            eprintln!("  [warn] {}:{}: {}", path.display(), w.line, w.message);
        }
        match result {
            Ok(rbs_content) => {
                if !SentinelTranspiler::has_content(&rbs_content) {
                    skipped.fetch_add(1, Ordering::Relaxed);
                    return;
                }

                for plugin in Plugin::ALL {
                    let issues = plugin.check(&rbs_content);
                    if !issues.is_empty() {
                        eprintln!(
                            "  [{}] {:?}:",
                            plugin.name(),
                            path.file_name().unwrap_or_default()
                        );
                        for (method, msg) in issues {
                            eprintln!("   - Method `{}`: {}", method, msg);
                        }
                    }
                }

                let target_path = derive_sig_path(&app_root, path, output_path);

                if let Some(parent) = target_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                if let Err(e) = std::fs::write(&target_path, &rbs_content) {
                    eprintln!("Failed to write {:?}: {}", target_path, e);
                    failed.fetch_add(1, Ordering::Relaxed);
                } else {
                    success.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to transpile {:?}: {}",
                    path.file_name().unwrap_or_default(),
                    e
                );
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    let elapsed = start.elapsed();
    let ok = success.load(Ordering::Relaxed);
    let err = failed.load(Ordering::Relaxed);

    let skip = skipped.load(Ordering::Relaxed);

    println!(
        "Generated {} RBS files in {:.1?} ({} skipped, {} errors)",
        ok, elapsed, skip, err
    );
}

pub fn derive_sig_path(app_root: &Path, rb_path: &Path, output_path: &Path) -> PathBuf {
    let mut p = output_path.to_path_buf();
    if let Ok(relative) = rb_path.strip_prefix(app_root) {
        p.push(relative);
    } else if let Some(name) = rb_path.file_name() {
        p.push(name);
    }
    p.set_extension("rbs");
    p
}
