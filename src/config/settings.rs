//! Configuration file support for kueue-dev

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    #[serde(default)]
    pub defaults: Defaults,

    #[serde(default)]
    pub colors: Colors,

    #[serde(default)]
    pub behavior: Behavior,
}

/// Default values for common operations
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Defaults {
    #[serde(default = "default_cluster_name")]
    pub cluster_name: String,

    #[serde(default = "default_cni_provider")]
    pub cni_provider: String,

    #[serde(default = "default_images_file")]
    pub images_file: String,
}

/// Color and theme settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Colors {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_theme")]
    pub theme: String,
}

/// Behavior settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Behavior {
    #[serde(default = "default_true")]
    pub confirm_destructive: bool,

    #[serde(default = "default_true")]
    pub parallel_operations: bool,

    #[serde(default = "default_true")]
    pub show_progress: bool,
}

// Default value functions
fn default_cluster_name() -> String {
    "kueue-test".to_string()
}

fn default_cni_provider() -> String {
    "calico".to_string()
}

fn default_images_file() -> String {
    "related_images.rphillips.json".to_string()
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            cluster_name: default_cluster_name(),
            cni_provider: default_cni_provider(),
            images_file: default_images_file(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            theme: default_theme(),
        }
    }
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            confirm_destructive: default_true(),
            parallel_operations: default_true(),
            show_progress: default_true(),
        }
    }
}

impl Settings {
    /// Load settings from file or return defaults
    pub fn load() -> Self {
        if let Some(path) = Self::find_config_file() {
            Self::load_from_file(&path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Load settings from a specific file
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let settings: Settings = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(settings)
    }

    /// Find config file in standard locations
    /// Priority:
    /// 1. .kueue-dev.toml in current directory
    /// 2. ~/.config/kueue-dev/config.toml (XDG config directory)
    fn find_config_file() -> Option<PathBuf> {
        // Check current directory
        let local_config = PathBuf::from(".kueue-dev.toml");
        if local_config.exists() {
            return Some(local_config);
        }

        // Check XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let xdg_config = config_dir.join("kueue-dev").join("config.toml");
            if xdg_config.exists() {
                return Some(xdg_config);
            }
        }

        None
    }

    /// Save settings to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let contents = toml::to_string_pretty(self).context("Failed to serialize settings")?;

        fs::write(path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Generate example config file content
    pub fn example_config() -> String {
        let example = Settings::default();
        let header = "# kueue-dev configuration file\n\
                      # Place this file at ~/.config/kueue-dev/config.toml or .kueue-dev.toml in your project\n\n";

        match toml::to_string_pretty(&example) {
            Ok(config) => format!("{}{}", header, config),
            Err(_) => {
                // Fallback in case serialization fails
                r#"# kueue-dev configuration file
# Place this file at ~/.config/kueue-dev/config.toml or .kueue-dev.toml in your project

[defaults]
cluster_name = "kueue-test"
cni_provider = "calico"
images_file = "related_images.rphillips.json"

[colors]
enabled = true
theme = "default"  # Options: default, dark, light, none

[behavior]
confirm_destructive = true
parallel_operations = true
show_progress = true
"#
                .to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.defaults.cluster_name, "kueue-test");
        assert_eq!(settings.defaults.cni_provider, "calico");
        assert!(settings.colors.enabled);
        assert!(settings.behavior.show_progress);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::default();
        let toml_str = toml::to_string_pretty(&settings).unwrap();
        assert!(toml_str.contains("cluster_name"));
        assert!(toml_str.contains("kueue-test"));
    }

    #[test]
    fn test_settings_deserialization() {
        let toml_str = r#"
[defaults]
cluster_name = "my-cluster"
cni_provider = "default"

[colors]
enabled = false

[behavior]
confirm_destructive = false
"#;
        let settings: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(settings.defaults.cluster_name, "my-cluster");
        assert_eq!(settings.defaults.cni_provider, "default");
        assert!(!settings.colors.enabled);
        assert!(!settings.behavior.confirm_destructive);
    }

    #[test]
    fn test_example_config() {
        let example = Settings::example_config();
        assert!(example.contains("kueue-dev configuration"));
        assert!(example.contains("[defaults]"));
        assert!(example.contains("[colors]"));
        assert!(example.contains("[behavior]"));
    }
}
