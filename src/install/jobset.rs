//! JobSet installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install JobSet
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing JobSet {}...", version);

    let jobset_url = format!(
        "https://github.com/kubernetes-sigs/jobset/releases/download/{}/manifests.yaml",
        version
    );

    // Check if JobSet namespace already exists
    let ns_check = kubectl::run_kubectl_output(&["get", "namespace", "jobset-system"], kubeconfig);

    if ns_check.is_ok() {
        crate::log_info!("JobSet namespace already exists, skipping installation");
        return Ok(());
    }

    crate::log_info!("Downloading JobSet manifest...");

    // Download and apply the manifest
    let jobset_yaml = reqwest::blocking::get(&jobset_url)
        .context("Failed to download JobSet manifest")?
        .text()
        .context("Failed to read JobSet manifest")?;

    // Use server-side apply to avoid annotation size limits for large CRDs
    kubectl::apply_yaml_server_side(&jobset_yaml, kubeconfig)
        .context("Failed to apply JobSet manifest")?;

    crate::log_info!("Waiting for JobSet controller to be ready...");

    // Wait for deployment to be available
    kubectl::wait_for_condition(
        "deployment/jobset-controller-manager",
        "condition=Available",
        Some("jobset-system"),
        "300s",
        kubeconfig,
    )
    .context("JobSet controller deployment not ready")?;

    crate::log_info!("JobSet installed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_jobset_module() {
        // Basic compile test
    }
}
