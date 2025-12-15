//! Kubeflow Training Operator installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install Kubeflow Training Operator (standalone mode)
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing Kubeflow Training Operator {}...", version);

    // Check if Training Operator namespace already exists
    let ns_check = kubectl::run_kubectl_output(&["get", "namespace", "kubeflow"], kubeconfig);

    if ns_check.is_ok() {
        crate::log_info!(
            "Kubeflow namespace already exists, skipping Training Operator installation"
        );
        return Ok(());
    }

    // Use kustomize to install from GitHub
    // The training-operator uses kustomize overlays for installation
    let kustomize_url = format!(
        "github.com/kubeflow/training-operator.git/manifests/overlays/standalone?ref={}",
        version
    );

    crate::log_info!(
        "Applying Training Operator manifest from: {}",
        kustomize_url
    );

    // Use kubectl apply -k for kustomize-based installation
    kubectl::run_kubectl(
        &["apply", "--server-side", "-k", &kustomize_url],
        kubeconfig,
    )
    .context("Failed to apply Training Operator manifest")?;

    crate::log_info!("Waiting for Training Operator controller to be ready...");

    // Wait for deployment to be available
    kubectl::wait_for_condition(
        "deployment/training-operator",
        "condition=Available",
        Some("kubeflow"),
        "300s",
        kubeconfig,
    )
    .context("Training Operator deployment not ready")?;

    crate::log_info!("Kubeflow Training Operator installed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_training_operator_module() {
        // Basic compile test
    }
}
