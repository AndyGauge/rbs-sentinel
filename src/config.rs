use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = ".sentinel.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelConfig {
    pub folders: Vec<String>,
    pub output: String,
    #[serde(default = "default_shared_paths")]
    pub shared_paths: Vec<String>,
    /// Emit `class Foo < Bar` instead of a bare `class Foo`.
    ///
    /// Off by default because it is not a no-op for existing projects: once Steep
    /// can see a parent, it resolves inherited methods and class-level DSL calls
    /// that were previously invisible, which surfaces pre-existing type errors.
    /// That is the point of the flag, but it should be an explicit migration
    /// rather than something an upgrade does to you.
    #[serde(default)]
    pub emit_superclasses: bool,
}

fn default_shared_paths() -> Vec<String> {
    vec!["sig/shared".to_string()]
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            folders: vec!["app".to_string()],
            output: "sig/generated".to_string(),
            shared_paths: default_shared_paths(),
            emit_superclasses: false,
        }
    }
}

impl SentinelConfig {
    /// Load config from .sentinel.toml, or return defaults if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Path::new(CONFIG_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", CONFIG_FILE))?;
        let config: SentinelConfig =
            toml::from_str(&contents).with_context(|| format!("Failed to parse {}", CONFIG_FILE))?;
        Ok(config)
    }

    /// Save config to .sentinel.toml.
    pub fn save(&self) -> Result<()> {
        let contents =
            toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(CONFIG_FILE, contents)
            .with_context(|| format!("Failed to write {}", CONFIG_FILE))?;
        Ok(())
    }

    /// Ensure a .sentinel.toml exists; create with defaults if missing.
    pub fn ensure_exists() -> Result<Self> {
        let path = Path::new(CONFIG_FILE);
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            println!("Created {} with default folders: {:?}", CONFIG_FILE, config.folders);
            return Ok(config);
        }
        Self::load()
    }

    /// Add a folder to the config. Returns true if it was added (not a duplicate).
    pub fn add_folder(&mut self, folder: &str) -> bool {
        let normalized = folder.trim_end_matches('/').to_string();
        if self.folders.contains(&normalized) {
            return false;
        }
        self.folders.push(normalized);
        true
    }

    /// Remove a folder from the config. Returns true if it was found and removed.
    pub fn remove_folder(&mut self, folder: &str) -> bool {
        let normalized = folder.trim_end_matches('/').to_string();
        let before = self.folders.len();
        self.folders.retain(|f| f != &normalized);
        self.folders.len() < before
    }

    /// Return the output path.
    pub fn output_path(&self) -> PathBuf {
        PathBuf::from(&self.output)
    }

    /// Return folder paths.
    pub fn folder_paths(&self) -> Vec<PathBuf> {
        self.folders.iter().map(PathBuf::from).collect()
    }

    /// Return shared type paths.
    pub fn shared_type_paths(&self) -> Vec<PathBuf> {
        self.shared_paths.iter().map(PathBuf::from).collect()
    }
}
