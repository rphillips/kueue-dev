//! OpenShift deployment support

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

use crate::config::images::ImageConfig;
use crate::install::{cert_manager, jobset, leaderworkerset, operator};

const CERT_MANAGER_VERSION: &str = "v1.13.3";
const JOBSET_VERSION: &str = "v0.10.1";
const LEADERWORKERSET_VERSION: &str = "v0.7.0";

/// Verify OpenShift cluster connection
pub fn verify_connection() -> Result<()> {
    crate::log_info!("Verifying OpenShift cluster connection...");

    // Check if logged in
    let output = std::process::Command::new("oc")
        .args(&["whoami"])
        .output()
        .context("Failed to run 'oc whoami'. Is oc installed and are you logged in?")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Not logged into an OpenShift cluster. Please run 'oc login' first."
        ));
    }

    let current_user = String::from_utf8(output.stdout)?.trim().to_string();

    let output = std::process::Command::new("oc")
        .args(&["whoami", "--show-server"])
        .output()?;

    let cluster_url = String::from_utf8(output.stdout)?.trim().to_string();

    crate::log_info!("Connected to OpenShift cluster as: {}", current_user);
    crate::log_info!("Cluster URL: {}", cluster_url);

    // Check for cluster-admin permissions
    let output = std::process::Command::new("oc")
        .args(&["auth", "can-i", "*", "*", "--all-namespaces"])
        .output()?;

    if !output.status.success() {
        crate::log_warn!("Warning: You may not have cluster-admin permissions");
        crate::log_warn!("This script requires elevated permissions to install cert-manager and CRDs");

        if !crate::utils::confirm("Continue anyway?")? {
            crate::log_info!("Exiting...");
            std::process::exit(0);
        }
    }

    crate::log_info!("Cluster connection verified");
    Ok(())
}

/// Deploy to OpenShift cluster
pub fn deploy_openshift(
    images_file: String,
    skip_tests: bool,
) -> Result<()> {
    crate::log_info!("Starting kueue-operator deployment on OpenShift cluster...");

    // Verify connection
    verify_connection()?;

    // Get project root
    let project_root = get_project_root()?;

    // Load image configuration
    let images_path = if images_file.starts_with('/') {
        PathBuf::from(images_file)
    } else {
        project_root.join(&images_file)
    };

    crate::log_info!("Using images from: {}", images_path.display());

    let image_config = ImageConfig::load(&images_path)?;

    // Display images
    crate::log_info!("");
    crate::log_info!("Images to be used:");
    crate::log_info!("  Operator:     {}", image_config.operator()?);
    crate::log_info!("  Operand:      {}", image_config.operand()?);
    crate::log_info!("  Must-gather:  {}", image_config.must_gather()?);
    crate::log_info!("");

    // Install cert-manager
    cert_manager::install(CERT_MANAGER_VERSION, None)?;

    // Install JobSet
    jobset::install(JOBSET_VERSION, None)?;

    // Install LeaderWorkerSet
    leaderworkerset::install(LEADERWORKERSET_VERSION, None)?;

    // Install CRDs
    operator::install_crds(&project_root, None)?;

    // Install operator
    operator::install_operator(&project_root, &image_config, None)?;

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Deployment completed successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Current context: {}", get_current_context()?);
    crate::log_info!("Current user: {}", get_current_user()?);
    crate::log_info!("");
    crate::log_info!("To view operator logs:");
    crate::log_info!("  oc logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f");
    crate::log_info!("");

    if skip_tests {
        crate::log_info!("Skipping e2e tests (--skip-tests flag provided)");
    } else {
        crate::log_info!("To run tests:");
        crate::log_info!("  kueue-dev test run");
    }

    crate::log_info!("");
    crate::log_info!("To cleanup:");
    crate::log_info!("  kubectl delete namespace openshift-kueue-operator");
    crate::log_info!("  kubectl delete -f {}/deploy/crd/", project_root.display());
    crate::log_info!("");

    Ok(())
}

/// Get current kubectl/oc context
fn get_current_context() -> Result<String> {
    let output = std::process::Command::new("oc")
        .args(&["config", "current-context"])
        .output()?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get current user
fn get_current_user() -> Result<String> {
    let output = std::process::Command::new("oc")
        .args(&["whoami"])
        .output()?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get project root directory
fn get_project_root() -> Result<PathBuf> {
    let current_dir = env::current_dir()?;

    // Check if we're in kueue-dev directory
    if current_dir.file_name().and_then(|n| n.to_str()) == Some("kueue-dev") {
        // Go up one level to kueue-operator root
        if let Some(parent) = current_dir.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    // Otherwise use current directory
    Ok(current_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openshift_module() {
        // Basic compile test
        assert!(true);
    }
}
