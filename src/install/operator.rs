//! Kueue operator installation

use crate::config::images::ImageConfig;
use crate::config::kueue::KueueConfig;
use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install Kueue operator CRDs
pub fn install_crds(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing CRDs from deploy/crd...");

    let crd_dir = crate::utils::operator_source_join("deploy/crd");

    if !crd_dir.exists() {
        return Err(anyhow::anyhow!(
            "CRD directory not found: {}",
            crd_dir.display()
        ));
    }

    // Apply all CRD files in the directory
    kubectl::run_kubectl(&["apply", "-f", crd_dir.to_str().unwrap()], kubeconfig)
        .context("Failed to apply CRDs")?;

    crate::log_info!("CRDs installed successfully");
    Ok(())
}

/// Install Kueue operator
pub fn install_operator(image_config: &ImageConfig, kubeconfig: Option<&Path>) -> Result<()> {
    install_operator_with_config(image_config, None, kubeconfig)
}

/// Install Kueue operator with optional Kueue CR configuration
pub fn install_operator_with_config(
    image_config: &ImageConfig,
    kueue_config: Option<&KueueConfig>,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    crate::log_info!("Installing kueue-operator...");

    // Get images from config
    let operator_image = image_config.operator()?;
    let operand_image = image_config.operand()?;
    let must_gather_image = image_config.must_gather()?;

    crate::log_info!("Using operator image: {}", operator_image);
    crate::log_info!("Using operand image: {}", operand_image);
    crate::log_info!("Using must-gather image: {}", must_gather_image);

    // Create temporary directory for modified manifests
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let temp_path = temp_dir.path();

    crate::log_info!(
        "Creating temporary deployment files in {}...",
        temp_path.display()
    );

    // Copy deploy files to temp directory
    copy_deploy_files(temp_path)?;

    // Update deployment file with images
    update_deployment_images(temp_path, operator_image, operand_image, must_gather_image)?;

    // Apply manifests in order
    apply_operator_manifests(temp_path, kubeconfig)?;

    // Wait for operator deployment to be available
    crate::log_info!("Waiting for operator deployment to be available...");
    kubectl::wait_for_condition(
        "deployment/openshift-kueue-operator",
        "condition=Available",
        Some("openshift-kueue-operator"),
        "300s",
        kubeconfig,
    )
    .context("Operator deployment not available")?;

    crate::log_info!("Operator deployment is available");

    // Give the operator time to start its controllers and be ready to reconcile
    crate::log_info!("Waiting for operator controllers to be ready...");
    std::thread::sleep(std::time::Duration::from_secs(30));

    // Create Kueue CR if config provided
    if let Some(config) = kueue_config {
        create_kueue_cr(config, kubeconfig)?;
    }

    Ok(())
}

/// Create Kueue CR from configuration
pub fn create_kueue_cr(config: &KueueConfig, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Creating Kueue CR: {}/{}", config.namespace, config.name);

    let yaml = config.to_yaml();

    kubectl::apply_yaml(&yaml, kubeconfig).context("Failed to create Kueue CR")?;

    crate::log_info!("Kueue CR created successfully");

    // Wait for operator to reconcile and create the kueue-controller-manager deployment
    crate::log_info!("Waiting for operator to create kueue-controller-manager deployment...");
    wait_for_deployment_to_exist(
        "kueue-controller-manager",
        &config.namespace,
        60,
        kubeconfig,
    )
    .context("Kueue controller-manager deployment was not created by operator")?;

    // Now wait for the deployment to be available
    crate::log_info!("Waiting for kueue-controller-manager deployment to be available...");
    kubectl::wait_for_condition(
        "deployment/kueue-controller-manager",
        "condition=Available",
        Some(&config.namespace),
        "300s",
        kubeconfig,
    )
    .context("Kueue controller-manager deployment did not become available")?;

    crate::log_info!("Kueue controller-manager deployment is available");

    Ok(())
}

/// Wait for a deployment to exist (be created by reconciliation)
fn wait_for_deployment_to_exist(
    name: &str,
    namespace: &str,
    timeout_secs: u64,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        // Check if deployment exists
        let result =
            kubectl::run_kubectl_output(&["get", "deployment", name, "-n", namespace], kubeconfig);

        if result.is_ok() {
            crate::log_info!("Deployment {} created", name);
            return Ok(());
        }

        // Check timeout
        if start.elapsed() >= timeout {
            return Err(anyhow::anyhow!(
                "Timeout waiting for deployment/{} to be created in namespace {}. \
                 The operator may not be reconciling the Kueue CR properly.",
                name,
                namespace
            ));
        }

        // Wait before retrying (poll every 5 seconds)
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Copy deploy files to temporary directory
fn copy_deploy_files(temp_dir: &Path) -> Result<()> {
    let deploy_dir = crate::utils::operator_source_join("deploy");

    if !deploy_dir.exists() {
        return Err(anyhow::anyhow!(
            "Deploy directory not found: {}",
            deploy_dir.display()
        ));
    }

    // Copy all .yaml files
    for entry in std::fs::read_dir(deploy_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let filename = path.file_name().unwrap();
            let dest = temp_dir.join(filename);
            std::fs::copy(&path, &dest)
                .with_context(|| format!("Failed to copy {:?}", filename))?;
        }
    }

    Ok(())
}

/// Update deployment file with image references
fn update_deployment_images(
    temp_dir: &Path,
    operator_image: &str,
    operand_image: &str,
    must_gather_image: &str,
) -> Result<()> {
    let deployment_file = temp_dir.join("07_deployment.yaml");

    if !deployment_file.exists() {
        return Err(anyhow::anyhow!(
            "Deployment file not found: {}",
            deployment_file.display()
        ));
    }

    let content = std::fs::read_to_string(&deployment_file)?;

    // Replace operator image
    let content = content.replace(
        "image: registry.redhat.io/kueue/kueue-rhel9-operator:latest",
        &format!("image: {}", operator_image),
    );

    // Replace operand image in env vars
    let content = content.replace(
        "value: registry.redhat.io/kueue/kueue-rhel9:latest",
        &format!("value: {}", operand_image),
    );

    // Replace must-gather image in env vars
    let content = content.replace(
        "value: registry.redhat.io/kueue/kueue-must-gather-rhel9:latest",
        &format!("value: {}", must_gather_image),
    );

    // Set imagePullPolicy to IfNotPresent for kind clusters
    let content = content.replace("imagePullPolicy: Always", "imagePullPolicy: IfNotPresent");

    std::fs::write(&deployment_file, content)?;

    // Verify replacements worked
    let final_content = std::fs::read_to_string(&deployment_file)?;
    if !final_content.contains(operator_image) {
        return Err(anyhow::anyhow!(
            "Failed to update operator image in deployment file"
        ));
    }
    if !final_content.contains(operand_image) {
        return Err(anyhow::anyhow!(
            "Failed to update operand image in deployment file"
        ));
    }
    if !final_content.contains(must_gather_image) {
        return Err(anyhow::anyhow!(
            "Failed to update must-gather image in deployment file"
        ));
    }

    crate::log_info!("Deployment file updated with images");
    crate::log_info!("  Operator image: {}", operator_image);
    crate::log_info!("  Operand image: {}", operand_image);
    crate::log_info!("  Must-gather image: {}", must_gather_image);

    Ok(())
}

/// Apply operator manifests in order
fn apply_operator_manifests(temp_dir: &Path, kubeconfig: Option<&Path>) -> Result<()> {
    let manifests = vec![
        "01_namespace.yaml",
        "02_clusterrole.yaml",
        "02_role.yaml",
        "03_clusterrolebinding.yaml",
        "03_rolebinding.yaml",
        "04_serviceaccount.yaml",
        "05_clusterrole_kueue-batch.yaml",
        "06_clusterrole_kueue-admin.yaml",
        "07_deployment.yaml",
    ];

    for manifest in manifests {
        let manifest_path = temp_dir.join(manifest);
        if !manifest_path.exists() {
            crate::log_warn!("Manifest not found: {}, skipping", manifest);
            continue;
        }

        crate::log_info!("Applying {}...", manifest);
        kubectl::run_kubectl(
            &["apply", "-f", manifest_path.to_str().unwrap()],
            kubeconfig,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_module() {
        // Basic compile test
        assert!(true);
    }
}
