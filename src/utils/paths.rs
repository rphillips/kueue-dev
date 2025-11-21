//! Path utilities for kueue-dev

use crate::config::settings::Settings;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::OnceLock;

// Global variable to store the CLI-provided operator source path
static CLI_OPERATOR_SOURCE: OnceLock<Option<String>> = OnceLock::new();

/// Set the operator source path from CLI argument
/// This should be called early in main() before any commands run
pub fn set_cli_operator_source(path: Option<String>) {
    CLI_OPERATOR_SOURCE.get_or_init(|| path);
}

/// Get the kueue-operator source path from CLI, config, or return None
/// Priority: CLI argument > Config file
pub fn get_operator_source_path() -> Option<PathBuf> {
    // First check CLI argument
    if let Some(Some(path)) = CLI_OPERATOR_SOURCE.get() {
        return Some(PathBuf::from(path));
    }

    // Fall back to config file
    let settings = Settings::load();
    settings
        .defaults
        .kueue_operator_source_path
        .as_ref()
        .map(PathBuf::from)
}

/// Ensure we're in the operator source directory
/// Changes current directory to the operator source path if needed
pub fn ensure_operator_source_directory() -> Result<PathBuf> {
    let source_path = get_operator_source_path().ok_or_else(|| {
        anyhow::anyhow!(
            "Kueue-operator source path is not configured.\n\
             Please set kueue_operator_source_path in your .kueue-dev.toml configuration file.\n\
             \n\
             Example configuration (~/.kueue-dev.toml or .kueue-dev.toml):\n\
             [defaults]\n\
             kueue_operator_source_path = \"/home/user/work/kueue-operator\""
        )
    })?;

    // Verify the path exists
    if !source_path.exists() {
        return Err(anyhow::anyhow!(
            "Kueue-operator source path does not exist: {}\n\
             Please update kueue_operator_source_path in your .kueue-dev.toml configuration",
            source_path.display()
        ));
    }

    // Verify it's a directory
    if !source_path.is_dir() {
        return Err(anyhow::anyhow!(
            "Kueue-operator source path is not a directory: {}\n\
             Please update kueue_operator_source_path in your .kueue-dev.toml configuration",
            source_path.display()
        ));
    }

    // Change to the operator source directory
    std::env::set_current_dir(&source_path).with_context(|| {
        format!(
            "Failed to change directory to operator source: {}",
            source_path.display()
        )
    })?;

    crate::log_info!("Working directory: {}", source_path.display());

    Ok(source_path)
}

/// Join a path relative to the operator source directory
/// If operator source path is configured, uses that path.
/// Otherwise, assumes we're already in the operator source directory and uses current dir.
pub fn operator_source_join(relative_path: &str) -> PathBuf {
    if let Some(source_path) = get_operator_source_path() {
        source_path.join(relative_path)
    } else {
        // If not configured, assume we're already in the operator source directory
        // This happens after ensure_operator_source_directory() has changed the cwd
        std::env::current_dir()
            .expect("Failed to get current directory")
            .join(relative_path)
    }
}

/// Get a path relative to the operator source directory as a Path reference
/// If operator source path is configured, uses that path.
/// Otherwise, assumes we're already in the operator source directory and uses current dir.
pub fn operator_source_path(relative_path: &str) -> PathBuf {
    operator_source_join(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_source_path() {
        let path = operator_source_join("deploy");
        assert!(path.ends_with("deploy"));
    }

    #[test]
    fn test_get_operator_source_path() {
        // This test may return None if config is not set, which is valid
        let _path = get_operator_source_path();
        // No assertion - just verify it doesn't panic
    }
}
