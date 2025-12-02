//! Kind cluster management operations

use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct KindCluster {
    pub name: String,
    pub cni_provider: CniProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CniProvider {
    Calico,
    Default,
}

impl FromStr for CniProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "calico" => Ok(CniProvider::Calico),
            "default" => Ok(CniProvider::Default),
            _ => Err(anyhow!(
                "Invalid CNI provider: {}. Must be 'calico' or 'default'",
                s
            )),
        }
    }
}

impl std::fmt::Display for CniProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CniProvider::Calico => write!(f, "calico"),
            CniProvider::Default => write!(f, "default"),
        }
    }
}

impl KindCluster {
    pub fn new(name: impl Into<String>, cni_provider: CniProvider) -> Self {
        Self {
            name: name.into(),
            cni_provider,
        }
    }

    /// Check if this cluster exists
    pub fn exists(&self) -> Result<bool> {
        let output = Command::new("kind")
            .args(["get", "clusters"])
            .output()
            .context("Failed to list kind clusters")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to list kind clusters"));
        }

        let clusters = String::from_utf8(output.stdout)?;
        Ok(clusters.lines().any(|line| line.trim() == self.name))
    }

    /// Create the kind cluster with optional custom kubeconfig path
    pub fn create(&self) -> Result<Option<PathBuf>> {
        self.create_with_kubeconfig(None)
    }

    /// Create the kind cluster with custom kubeconfig path
    /// Returns Some(PathBuf) if kubeconfig is saved, None otherwise
    pub fn create_with_kubeconfig(&self, kubeconfig: Option<PathBuf>) -> Result<Option<PathBuf>> {
        crate::log_info!("Creating kind cluster '{}'...", self.name);
        crate::log_info!("Cluster will have 2 control-plane nodes and 2 worker nodes");

        if matches!(self.cni_provider, CniProvider::Calico) {
            crate::log_info!("CNI provider: calico");
            crate::log_info!("Note: Nodes will not be ready until Calico is installed");
        } else {
            crate::log_info!("CNI provider: default (kindnet)");
        }

        // Check if cluster already exists
        if self.exists()? {
            crate::log_warn!("Cluster '{}' already exists", self.name);

            if !crate::utils::confirm(&format!(
                "Do you want to delete and recreate cluster '{}'?",
                self.name
            ))? {
                crate::log_info!("Using existing cluster");
                // Export kubeconfig only if path was provided
                if kubeconfig.is_some() {
                    let kc_path = self.export_kubeconfig_with_custom(kubeconfig)?;
                    return Ok(Some(kc_path));
                } else {
                    return Ok(None);
                }
            }

            crate::log_info!("Deleting existing cluster...");
            self.delete()?;
        }

        // Generate kind config
        let config = self.generate_config();

        // Create cluster with config
        let mut cmd = Command::new("kind");
        cmd.args(["create", "cluster", "--name", &self.name, "--config", "-"]);

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn kind create cluster")?;

        // Write config to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(config.as_bytes())
                .context("Failed to write kind config")?;
        }

        let status = child
            .wait()
            .context("Failed to wait for kind create cluster")?;

        if !status.success() {
            return Err(anyhow!("Failed to create kind cluster"));
        }

        crate::log_info!("Cluster '{}' created successfully", self.name);

        // Export kubeconfig only if path was provided
        if kubeconfig.is_some() {
            let kubeconfig_path = self.export_kubeconfig_with_custom(kubeconfig)?;
            Ok(Some(kubeconfig_path))
        } else {
            crate::log_info!("Kubeconfig not saved (no path specified)");
            Ok(None)
        }
    }

    /// Delete the kind cluster
    pub fn delete(&self) -> Result<()> {
        crate::log_info!("Deleting kind cluster '{}'...", self.name);

        let status = Command::new("kind")
            .args(["delete", "cluster", "--name", &self.name])
            .status()
            .context("Failed to delete kind cluster")?;

        if !status.success() {
            return Err(anyhow!("Failed to delete kind cluster '{}'", self.name));
        }

        crate::log_info!("Cluster '{}' deleted successfully", self.name);
        Ok(())
    }

    /// List all kind clusters
    pub fn list_all() -> Result<Vec<String>> {
        let output = Command::new("kind")
            .args(["get", "clusters"])
            .output()
            .context("Failed to list kind clusters")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to list kind clusters"));
        }

        let clusters = String::from_utf8(output.stdout)?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(clusters)
    }

    /// Export kubeconfig to file with optional custom path
    fn export_kubeconfig_with_custom(&self, custom_path: Option<PathBuf>) -> Result<PathBuf> {
        let kubeconfig_path =
            custom_path.unwrap_or_else(|| crate::utils::operator_source_join("kube.kubeconfig"));

        crate::log_info!("Exporting kubeconfig to {}...", kubeconfig_path.display());

        let output = Command::new("kind")
            .args(["get", "kubeconfig", "--name", &self.name])
            .output()
            .context("Failed to get kind kubeconfig")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to get kubeconfig for cluster '{}'",
                self.name
            ));
        }

        std::fs::write(&kubeconfig_path, output.stdout)
            .context("Failed to write kubeconfig file")?;

        // Get the actual absolute path after writing
        let final_path = kubeconfig_path
            .canonicalize()
            .unwrap_or(kubeconfig_path.clone());
        crate::log_info!("KUBECONFIG written to: {}", final_path.display());

        Ok(final_path)
    }

    /// Generate kind cluster config YAML
    fn generate_config(&self) -> String {
        let disable_cni = matches!(self.cni_provider, CniProvider::Calico);

        format!(
            r#"kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
networking:
  disableDefaultCNI: {}
  podSubnet: "10.244.0.0/16"
  serviceSubnet: "10.96.0.0/16"
nodes:
- role: control-plane
  kubeadmConfigPatches:
  - |
    apiVersion: kubeadm.k8s.io/v1beta3
    kind: ClusterConfiguration
    apiServer:
      extraArgs:
        v: "4"
- role: control-plane
  kubeadmConfigPatches:
  - |
    apiVersion: kubeadm.k8s.io/v1beta3
    kind: ClusterConfiguration
    apiServer:
      extraArgs:
        v: "4"
- role: worker
- role: worker
"#,
            disable_cni
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cni_provider_from_str() {
        assert_eq!(
            CniProvider::from_str("calico").unwrap(),
            CniProvider::Calico
        );
        assert_eq!(
            CniProvider::from_str("default").unwrap(),
            CniProvider::Default
        );
        assert_eq!(
            CniProvider::from_str("Calico").unwrap(),
            CniProvider::Calico
        );
        assert!(CniProvider::from_str("invalid").is_err());
    }

    #[test]
    fn test_cni_provider_display() {
        assert_eq!(format!("{}", CniProvider::Calico), "calico");
        assert_eq!(format!("{}", CniProvider::Default), "default");
    }

    #[test]
    fn test_generate_config() {
        let cluster = KindCluster::new("test", CniProvider::Calico);
        let config = cluster.generate_config();
        assert!(config.contains("disableDefaultCNI: true"));
        assert!(config.contains("podSubnet: \"10.244.0.0/16\""));

        let cluster = KindCluster::new("test", CniProvider::Default);
        let config = cluster.generate_config();
        assert!(config.contains("disableDefaultCNI: false"));
    }
}
