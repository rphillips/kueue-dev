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

    #[serde(default)]
    pub kueue: KueueSettings,

    #[serde(default)]
    pub tests: TestSettings,

    #[serde(default)]
    pub versions: Versions,
}

/// Test configuration settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestSettings {
    #[serde(default = "default_operator_skip_patterns")]
    pub operator_skip_patterns: Vec<String>,

    #[serde(default = "default_upstream_skip_patterns")]
    pub upstream_skip_patterns: Vec<String>,
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

    /// Optional path to kueue-operator source directory.
    /// If not set, the current working directory will be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kueue_operator_source_path: Option<String>,

    /// Optional path where kind should save kubeconfig files.
    /// If not set, kubeconfig will not be automatically saved to a file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kubeconfig_path: Option<String>,
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

/// Kueue CR configuration settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KueueSettings {
    #[serde(default = "default_kueue_name")]
    pub name: String,

    #[serde(default = "default_kueue_namespace")]
    pub namespace: String,

    #[serde(default = "default_kueue_frameworks")]
    pub frameworks: Vec<String>,
}

/// Version settings for dependencies
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Versions {
    #[serde(default = "default_cert_manager_version")]
    pub cert_manager: String,

    #[serde(default = "default_jobset_version")]
    pub jobset: String,

    #[serde(default = "default_leaderworkerset_version")]
    pub leaderworkerset: String,

    #[serde(default = "default_calico_version")]
    pub calico: String,

    #[serde(default = "default_prometheus_operator_version")]
    pub prometheus_operator: String,
}

// Default value functions
fn default_cluster_name() -> String {
    "kueue-test".to_string()
}

fn default_cni_provider() -> String {
    "calico".to_string()
}

fn default_images_file() -> String {
    "related_images.json".to_string()
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_kueue_name() -> String {
    "cluster".to_string()
}

fn default_kueue_namespace() -> String {
    "openshift-kueue-operator".to_string()
}

fn default_kueue_frameworks() -> Vec<String> {
    vec![
        "BatchJob".to_string(),
        "Pod".to_string(),
        "Deployment".to_string(),
        "StatefulSet".to_string(),
        "JobSet".to_string(),
        "LeaderWorkerSet".to_string(),
    ]
}

fn default_cert_manager_version() -> String {
    "v1.18.0".to_string()
}

fn default_jobset_version() -> String {
    "v0.10.1".to_string()
}

fn default_leaderworkerset_version() -> String {
    "v0.7.0".to_string()
}

fn default_calico_version() -> String {
    "v3.28.2".to_string()
}

fn default_prometheus_operator_version() -> String {
    "v0.82.2".to_string()
}

fn default_operator_skip_patterns() -> Vec<String> {
    vec![
        "AppWrapper".to_string(),
        "PyTorch".to_string(),
        "JobSet".to_string(),
        "LeaderWorkerSet".to_string(),
        "JAX".to_string(),
        "Kuberay".to_string(),
        "Metrics".to_string(),
        "Fair".to_string(),
        "TopologyAwareScheduling".to_string(),
        "Kueue visibility server".to_string(),
        "Failed Pod can be replaced in group".to_string(),
        "should allow to schedule a group of diverse pods".to_string(),
        "StatefulSet created with WorkloadPriorityClass".to_string(),
    ]
}

fn default_upstream_skip_patterns() -> Vec<String> {
    vec![
        // do not deploy AppWrapper in OCP
        "AppWrapper".to_string(),
        // do not deploy PyTorch in OCP
        "PyTorch".to_string(),
        // do not deploy JobSet in OCP
        "TrainJob".to_string(),
        // do not deploy LWS in OCP
        "JAX".to_string(),
        // do not deploy KubeRay in OCP
        "Kuberay".to_string(),
        // metrics setup is different than our OCP setup
        "Metrics".to_string(),
        // ring -> we do not enable Fair sharing by default in our operator
        "Fair".to_string(),
        // we do not enable this feature in our operator
        "TopologyAwareScheduling".to_string(),
        // relies on particular CPU setup to force pods to not schedule
        "Failed Pod can be replaced in group".to_string(),
        // relies on particular CPU setup
        "should allow to schedule a group of diverse pods".to_string(),
        // relies on particular CPU setup
        "StatefulSet created with WorkloadPriorityClass".to_string(),
        // We do not have kueuectl in our operator
        "Kueuectl".to_string(),
    ]
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            cluster_name: default_cluster_name(),
            cni_provider: default_cni_provider(),
            images_file: default_images_file(),
            kueue_operator_source_path: None,
            kubeconfig_path: None,
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

impl Default for KueueSettings {
    fn default() -> Self {
        Self {
            name: default_kueue_name(),
            namespace: default_kueue_namespace(),
            frameworks: default_kueue_frameworks(),
        }
    }
}

impl Default for TestSettings {
    fn default() -> Self {
        Self {
            operator_skip_patterns: default_operator_skip_patterns(),
            upstream_skip_patterns: default_upstream_skip_patterns(),
        }
    }
}

impl Default for Versions {
    fn default() -> Self {
        Self {
            cert_manager: default_cert_manager_version(),
            jobset: default_jobset_version(),
            leaderworkerset: default_leaderworkerset_version(),
            calico: default_calico_version(),
            prometheus_operator: default_prometheus_operator_version(),
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
images_file = "related_images.json"
# kueue_operator_source_path = "/path/to/kueue-operator"  # Optional: Path to kueue-operator source. Defaults to current directory.
# kubeconfig_path = "kube.kubeconfig"  # Optional: Path where kind should save kubeconfig. If not set, kubeconfig won't be saved to file.

[colors]
enabled = true
theme = "default"  # Options: default, dark, light, none

[behavior]
confirm_destructive = true
parallel_operations = true
show_progress = true

[kueue]
# Kueue CR name - should always be "cluster"
name = "cluster"
# Kueue CR namespace - should always be "openshift-kueue-operator"
namespace = "openshift-kueue-operator"
# Frameworks to enable
frameworks = ["BatchJob", "Pod", "Deployment", "StatefulSet", "JobSet", "LeaderWorkerSet"]

[tests]
# Test patterns to skip for operator tests
operator_skip_patterns = [
    "AppWrapper",
    "PyTorch",
    "JobSet",
    "LeaderWorkerSet",
    "JAX",
    "Kuberay",
    "Metrics",
    "Fair",
    "TopologyAwareScheduling",
    "Kueue visibility server",
    "Failed Pod can be replaced in group",
    "should allow to schedule a group of diverse pods",
    "StatefulSet created with WorkloadPriorityClass"
]

# Test patterns to skip for upstream tests
upstream_skip_patterns = [
    "AppWrapper",
    "PyTorch",
    "TrainJob",
    "JAX",
    "Kuberay",
    "Metrics",
    "Fair",
    "TopologyAwareScheduling",
    "Failed Pod can be replaced in group",
    "should allow to schedule a group of diverse pods",
    "StatefulSet created with WorkloadPriorityClass",
    "Kueuectl"
]

[versions]
# Version of cert-manager to install
cert_manager = "v1.18.0"
# Version of JobSet to install
jobset = "v0.10.1"
# Version of LeaderWorkerSet to install
leaderworkerset = "v0.7.0"
# Version of Calico CNI to install
calico = "v3.28.2"
# Version of Prometheus Operator to install
prometheus_operator = "v0.82.2"
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
