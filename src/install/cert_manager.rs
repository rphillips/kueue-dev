//! cert-manager installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install cert-manager
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing cert-manager {}...", version);

    let cert_manager_url = format!(
        "https://github.com/cert-manager/cert-manager/releases/download/{}/cert-manager.yaml",
        version
    );

    // Check if cert-manager namespace already exists
    let ns_check = kubectl::run_kubectl_output(&["get", "namespace", "cert-manager"], kubeconfig);

    if ns_check.is_ok() {
        crate::log_info!("cert-manager namespace already exists, skipping installation");
        return Ok(());
    }

    crate::log_info!("Downloading cert-manager manifest...");

    // Download and apply the manifest
    let cert_manager_yaml = reqwest::blocking::get(&cert_manager_url)
        .context("Failed to download cert-manager manifest")?
        .text()
        .context("Failed to read cert-manager manifest")?;

    kubectl::apply_yaml(&cert_manager_yaml, kubeconfig)
        .context("Failed to apply cert-manager manifest")?;

    crate::log_info!("Waiting for cert-manager to be ready...");

    // Wait for deployments to be available
    kubectl::wait_for_condition(
        "deployment/cert-manager",
        "condition=Available",
        Some("cert-manager"),
        "300s",
        kubeconfig,
    )
    .context("cert-manager deployment not ready")?;

    kubectl::wait_for_condition(
        "deployment/cert-manager-webhook",
        "condition=Available",
        Some("cert-manager"),
        "300s",
        kubeconfig,
    )
    .context("cert-manager-webhook deployment not ready")?;

    kubectl::wait_for_condition(
        "deployment/cert-manager-cainjector",
        "condition=Available",
        Some("cert-manager"),
        "300s",
        kubeconfig,
    )
    .context("cert-manager-cainjector deployment not ready")?;

    crate::log_info!("cert-manager installed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_manager_module() {
        // Basic compile test
        assert!(true);
    }
}
