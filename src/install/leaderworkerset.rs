//! LeaderWorkerSet installation

use anyhow::{Context, Result};
use std::path::Path;
use crate::k8s::kubectl;

/// Install LeaderWorkerSet
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing LeaderWorkerSet {}...", version);

    let lws_url = format!(
        "https://github.com/kubernetes-sigs/lws/releases/download/{}/manifests.yaml",
        version
    );

    // Check if LeaderWorkerSet namespace already exists
    let ns_check = kubectl::run_kubectl_output(&["get", "namespace", "lws-system"], kubeconfig);

    if ns_check.is_ok() {
        crate::log_info!("LeaderWorkerSet namespace already exists, skipping installation");
        return Ok(());
    }

    crate::log_info!("Downloading LeaderWorkerSet manifest...");

    // Download and apply the manifest
    let lws_yaml = reqwest::blocking::get(&lws_url)
        .context("Failed to download LeaderWorkerSet manifest")?
        .text()
        .context("Failed to read LeaderWorkerSet manifest")?;

    kubectl::apply_yaml(&lws_yaml, kubeconfig)
        .context("Failed to apply LeaderWorkerSet manifest")?;

    crate::log_info!("Waiting for LeaderWorkerSet controller to be ready...");

    // Wait for deployment to be available
    kubectl::wait_for_condition(
        "deployment/lws-controller-manager",
        "condition=Available",
        Some("lws-system"),
        "300s",
        kubeconfig,
    ).context("LeaderWorkerSet controller deployment not ready")?;

    crate::log_info!("LeaderWorkerSet installed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaderworkerset_module() {
        // Basic compile test
        assert!(true);
    }
}
