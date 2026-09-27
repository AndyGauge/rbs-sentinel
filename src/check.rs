use crate::init::derive_sig_path;
use crate::plugin::Plugin;
use crate::transpiler::SentinelTranspiler;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use walkdir::WalkDir;

pub fn run(app_path: &Path, output_path: &Path, shared_paths: &[PathBuf], emit_superclasses: bool) -> bool {
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
    println!("Checking {} Ruby files in {:?}", total, app_path);

    let stale_files: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let missing_files: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let checked = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    files.par_iter().for_each(|path| {
        let mut transpiler = SentinelTranspiler::new();
        transpiler.set_emit_superclasses(emit_superclasses);
        transpiler.set_shared_paths(shared_paths.to_vec());

        match transpiler.transpile_file(path) {
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

                if !target_path.exists() {
                    missing_files.lock().unwrap().push(target_path);
                    checked.fetch_add(1, Ordering::Relaxed);
                    return;
                }

                match std::fs::read_to_string(&target_path) {
                    Ok(existing) if existing == rbs_content => {
                        checked.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(_) => {
                        stale_files.lock().unwrap().push(target_path);
                        checked.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("Failed to read {:?}: {}", target_path, e);
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
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
    let stale = stale_files.lock().unwrap();
    let missing = missing_files.lock().unwrap();
    let err = failed.load(Ordering::Relaxed);
    let skip = skipped.load(Ordering::Relaxed);

    let all_ok = stale.is_empty() && missing.is_empty() && err == 0;

    if !missing.is_empty() {
        println!("\nMissing RBS files:");
        for f in missing.iter() {
            println!("  {}", f.display());
        }
    }

    if !stale.is_empty() {
        println!("\nStale RBS files:");
        for f in stale.iter() {
            println!("  {}", f.display());
        }
    }

    if all_ok {
        println!(
            "All signatures up to date ({} checked, {} skipped) in {:.1?}",
            checked.load(Ordering::Relaxed),
            skip,
            elapsed
        );
    } else {
        println!(
            "\n{} missing, {} stale, {} errors ({} skipped) in {:.1?}",
            missing.len(),
            stale.len(),
            err,
            skip,
            elapsed
        );
        println!("Run `bundle exec sentinel init` to regenerate.");
    }

    all_ok
}
