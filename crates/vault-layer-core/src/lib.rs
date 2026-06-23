//! Core path and runtime primitives for VaultLayer.
//!
//! The vault remains the source of truth. Runtime indexes, databases, caches,
//! and embeddings live outside the repository and outside the vault by default.

use std::env;
use std::path::{Path, PathBuf};

/// Default runtime directory relative to the user's home directory.
pub const DEFAULT_STATE_SUBDIR: &str = ".local/share/vault-layer";

/// Commands planned for the first public CLI surface.
pub const COMMANDS: &[&str] = &["init", "index", "search", "context", "serve"];

/// Minimal runtime configuration resolved by the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub vault_path: PathBuf,
    pub state_dir: PathBuf,
}

impl RuntimeConfig {
    /// Build a config from a vault path and an optional explicit state directory.
    pub fn new(vault_path: impl Into<PathBuf>, state_dir: Option<PathBuf>) -> Result<Self, String> {
        let vault_path = vault_path.into();
        if vault_path.as_os_str().is_empty() {
            return Err("vault path cannot be empty".to_string());
        }
        let state_dir = match state_dir {
            Some(path) => path,
            None => default_state_dir()?,
        };
        if is_inside(&state_dir, &vault_path) {
            return Err(format!(
                "state directory must be outside the vault: {} is inside {}",
                state_dir.display(),
                vault_path.display()
            ));
        }
        Ok(Self { vault_path, state_dir })
    }

    /// Planned local database path for a named vault id.
    pub fn database_path(&self, vault_id: &str) -> PathBuf {
        self.state_dir.join(vault_id).join("vault-layer.db")
    }
}

/// Resolve the default user-state directory without touching the filesystem.
pub fn default_state_dir() -> Result<PathBuf, String> {
    match env::var_os("VAULT_LAYER_STATE_DIR") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => {
            let home = env::var_os("HOME").ok_or_else(|| "HOME is not set; pass --state-dir".to_string())?;
            Ok(PathBuf::from(home).join(DEFAULT_STATE_SUBDIR))
        }
    }
}

/// Return true when `child` is equal to or nested under `parent` lexically.
pub fn is_inside(child: &Path, parent: &Path) -> bool {
    let child_components: Vec<_> = child.components().collect();
    let parent_components: Vec<_> = parent.components().collect();
    child_components.starts_with(&parent_components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_database_path_uses_external_state_dir() {
        let config = RuntimeConfig::new(
            "/tmp/example-vault",
            Some(PathBuf::from("/tmp/vault-layer-state")),
        )
        .expect("valid config");
        assert_eq!(
            config.database_path("demo"),
            PathBuf::from("/tmp/vault-layer-state/demo/vault-layer.db")
        );
    }

    #[test]
    fn rejects_state_dir_inside_vault() {
        let error = RuntimeConfig::new(
            "/tmp/example-vault",
            Some(PathBuf::from("/tmp/example-vault/.vault-layer")),
        )
        .expect_err("state inside vault must be rejected");
        assert!(error.contains("outside the vault"));
    }

    #[test]
    fn command_surface_mentions_first_mvp_commands() {
        for command in ["init", "index", "search", "context", "serve"] {
            assert!(COMMANDS.contains(&command));
        }
    }
}
