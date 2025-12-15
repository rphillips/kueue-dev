//! AppWrapper installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install AppWrapper
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing AppWrapper {}...", version);

    let appwrapper_url = format!(
        "https://github.com/project-codeflare/appwrapper/releases/download/{}/install.yaml",
        version
    );

    // Check if AppWrapper namespace already exists
    let ns_check =
        kubectl::run_kubectl_output(&["get", "namespace", "appwrapper-system"], kubeconfig);

    if ns_check.is_ok() {
        crate::log_info!("AppWrapper namespace already exists, skipping installation");
        return Ok(());
    }

    crate::log_info!("Downloading AppWrapper manifest...");

    // Download and apply the manifest
    let appwrapper_yaml = reqwest::blocking::get(&appwrapper_url)
        .context("Failed to download AppWrapper manifest")?
        .text()
        .context("Failed to read AppWrapper manifest")?;

    // Use server-side apply to avoid annotation size limits for large CRDs
    kubectl::apply_yaml_server_side(&appwrapper_yaml, kubeconfig)
        .context("Failed to apply AppWrapper manifest")?;

    crate::log_info!("Waiting for AppWrapper controller to be ready...");

    // Wait for deployment to be available
    kubectl::wait_for_condition(
        "deployment/appwrapper-controller-manager",
        "condition=Available",
        Some("appwrapper-system"),
        "300s",
        kubeconfig,
    )
    .context("AppWrapper controller deployment not ready")?;

    crate::log_info!("AppWrapper installed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_appwrapper_module() {
        // Basic compile test
    }
}
