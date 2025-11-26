//! OLM (Operator Lifecycle Manager) installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Check if the kueue-operator is already installed
pub fn is_operator_installed(kubeconfig: Option<&Path>) -> bool {
    // Check if the operator namespace exists
    let namespace_check = kubectl::run_kubectl_output(
        &["get", "namespace", "openshift-kueue-operator"],
        kubeconfig,
    );

    if namespace_check.is_err() {
        return false;
    }

    // Check if the operator deployment or catalog source exists
    let deployment_check = kubectl::run_kubectl_output(
        &[
            "get",
            "deployment",
            "openshift-kueue-operator",
            "-n",
            "openshift-kueue-operator",
        ],
        kubeconfig,
    );

    let catalog_check = kubectl::run_kubectl_output(
        &[
            "get",
            "catalogsource",
            "kueue-operator-catalog",
            "-n",
            "openshift-kueue-operator",
        ],
        kubeconfig,
    );

    deployment_check.is_ok() || catalog_check.is_ok()
}

/// Uninstall the kueue-operator if it's installed via OLM cleanup
pub fn uninstall_operator_if_exists(kubeconfig: Option<&Path>) -> Result<()> {
    if !is_operator_installed(kubeconfig) {
        crate::log_info!("No existing operator installation detected");
        return Ok(());
    }

    crate::log_info!("Existing operator installation detected, uninstalling via OLM...");

    // Use operator-sdk cleanup to properly remove OLM-managed resources
    if which::which("operator-sdk").is_err() {
        crate::log_warn!("operator-sdk not found, skipping cleanup");
        crate::log_warn!(
            "Install operator-sdk from: https://sdk.operatorframework.io/docs/installation/"
        );
        return Ok(());
    }

    // Run operator-sdk cleanup first to remove the operator deployment
    crate::log_info!("Running operator-sdk cleanup kueue-operator...");
    let mut cleanup_cmd = Command::new("operator-sdk");
    if let Some(kc) = kubeconfig {
        cleanup_cmd.env("KUBECONFIG", kc);
    }

    cleanup_cmd.args([
        "cleanup",
        "kueue-operator",
        "-n",
        "openshift-kueue-operator",
    ]);

    let cleanup_output = cleanup_cmd
        .output()
        .context("Failed to run operator-sdk cleanup")?;

    if cleanup_output.status.success() {
        crate::log_info!("operator-sdk cleanup completed successfully");
    } else {
        let stderr = String::from_utf8_lossy(&cleanup_output.stderr);
        let stdout = String::from_utf8_lossy(&cleanup_output.stdout);
        crate::log_warn!("operator-sdk cleanup output:\n{}\n{}", stdout, stderr);
    }

    // Wait for cleanup to complete
    crate::log_info!("Waiting for operator resources to be removed...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Verify cleanup - check if deployment is gone
    for i in 0..12 {
        // Try for up to 60 seconds
        let deployment_check = kubectl::run_kubectl_output(
            &[
                "get",
                "deployment",
                "openshift-kueue-operator",
                "-n",
                "openshift-kueue-operator",
            ],
            kubeconfig,
        );

        if deployment_check.is_err() {
            crate::log_info!("Operator deployment removed successfully");
            break;
        }

        if i < 11 {
            crate::log_info!("Waiting for operator deployment to be removed...");
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            crate::log_warn!("Operator deployment still exists after cleanup timeout");
        }
    }

    // Delete the namespace to clean up any remaining resources
    crate::log_info!("Removing operator namespace...");
    kubectl::run_kubectl(
        &[
            "delete",
            "namespace",
            "openshift-kueue-operator",
            "--ignore-not-found",
            "--timeout=60s",
        ],
        kubeconfig,
    )
    .ok();

    // Delete any remaining Kueue CRs as final cleanup
    crate::log_info!("Cleaning up any remaining Kueue CRs...");
    kubectl::run_kubectl(
        &[
            "delete",
            "kueue",
            "--all",
            "--all-namespaces",
            "--timeout=30s",
            "--ignore-not-found",
        ],
        kubeconfig,
    )
    .ok();

    crate::log_info!("Operator uninstall complete");
    Ok(())
}

/// Check if OLM is already installed
pub fn is_olm_installed(kubeconfig: Option<&Path>) -> bool {
    // Check if the olm namespace exists and has the expected deployments
    let namespace_check = kubectl::run_kubectl_output(&["get", "namespace", "olm"], kubeconfig);

    if namespace_check.is_err() {
        return false;
    }

    // Check if key OLM deployments exist
    let deployments = ["olm-operator", "catalog-operator"];
    for deployment in &deployments {
        let result = kubectl::run_kubectl_output(
            &["get", "deployment", deployment, "-n", "olm"],
            kubeconfig,
        );
        if result.is_err() {
            return false;
        }
    }

    true
}

/// Install OLM (Operator Lifecycle Manager)
pub fn install_olm(kubeconfig: Option<&Path>) -> Result<()> {
    // Check if OLM is already installed
    if is_olm_installed(kubeconfig) {
        crate::log_info!("OLM is already installed, skipping installation");
        return Ok(());
    }

    crate::log_info!("Installing latest OLM...");

    // Get the latest OLM release version from GitHub API
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://api.github.com/repos/operator-framework/operator-lifecycle-manager/releases/latest")
        .header("User-Agent", "kueue-dev")
        .send()
        .context("Failed to fetch OLM releases")?;

    let release: serde_json::Value = response.json()?;
    let olm_version = release["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get latest OLM version"))?;

    crate::log_info!("Installing OLM version: {}", olm_version);

    // Download CRDs and OLM manifests
    let crds_url = format!(
        "https://github.com/operator-framework/operator-lifecycle-manager/releases/download/{}/crds.yaml",
        olm_version
    );
    let olm_url = format!(
        "https://github.com/operator-framework/operator-lifecycle-manager/releases/download/{}/olm.yaml",
        olm_version
    );

    // Apply CRDs with server-side apply
    crate::log_info!("Applying OLM CRDs...");
    let crds_yaml = reqwest::blocking::get(&crds_url)
        .context("Failed to download OLM CRDs")?
        .text()?;

    let mut temp_crds = tempfile::NamedTempFile::new()?;
    use std::io::Write;
    temp_crds.write_all(crds_yaml.as_bytes())?;
    temp_crds.flush()?;

    kubectl::run_kubectl(
        &[
            "apply",
            "--server-side",
            "-f",
            temp_crds.path().to_str().unwrap(),
        ],
        kubeconfig,
    )?;

    // Apply OLM manifests
    crate::log_info!("Applying OLM manifests...");
    let olm_yaml = reqwest::blocking::get(&olm_url)
        .context("Failed to download OLM manifests")?
        .text()?;

    let mut temp_olm = tempfile::NamedTempFile::new()?;
    temp_olm.write_all(olm_yaml.as_bytes())?;
    temp_olm.flush()?;

    kubectl::run_kubectl(
        &[
            "apply",
            "--server-side",
            "-f",
            temp_olm.path().to_str().unwrap(),
        ],
        kubeconfig,
    )?;

    crate::log_info!("Waiting for OLM to be ready...");

    // Wait for OLM deployments
    std::thread::sleep(std::time::Duration::from_secs(5));

    kubectl::wait_for_condition(
        "deployment/catalog-operator",
        "condition=Available",
        Some("olm"),
        "300s",
        kubeconfig,
    )
    .ok();

    kubectl::wait_for_condition(
        "deployment/olm-operator",
        "condition=Available",
        Some("olm"),
        "300s",
        kubeconfig,
    )
    .ok();

    kubectl::wait_for_condition(
        "deployment/packageserver",
        "condition=Available",
        Some("olm"),
        "300s",
        kubeconfig,
    )
    .ok();

    crate::log_info!("OLM installed successfully");
    Ok(())
}

/// Helper function to run operator-sdk run bundle with retry on catalog exists error
fn run_bundle_with_retry(bundle_image: &str, kubeconfig: Option<&Path>) -> Result<bool> {
    // Check if operator is already running (catalog source exists)
    let catalog_check = kubectl::run_kubectl_output(
        &[
            "get",
            "catalogsource",
            "kueue-operator-catalog",
            "-n",
            "openshift-kueue-operator",
        ],
        kubeconfig,
    );

    // If catalog source exists, go directly to cleanup and retry
    if catalog_check.is_ok() {
        crate::log_warn!("Operator catalog source already exists from previous deployment");
        crate::log_info!("Running cleanup before attempting installation...");

        return cleanup_and_retry(bundle_image, kubeconfig);
    }

    // Catalog doesn't exist, proceed with normal installation
    let mut cmd = Command::new("operator-sdk");
    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args([
        "run",
        "bundle",
        bundle_image,
        "--namespace",
        "openshift-kueue-operator",
        "--timeout",
        "10m",
    ]);

    let output = cmd.output().context("Failed to run operator-sdk")?;

    if output.status.success() {
        return Ok(true);
    }

    // Check if the error is due to catalog already existing
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stderr.contains("already exists") || stdout.contains("already exists") {
        return cleanup_and_retry(bundle_image, kubeconfig);
    }

    // For other errors, return the original error
    Err(anyhow::anyhow!(
        "operator-sdk run bundle failed:\n{}\n{}",
        stdout,
        stderr
    ))
}

/// Cleanup existing operator installation and retry bundle installation
fn cleanup_and_retry(bundle_image: &str, kubeconfig: Option<&Path>) -> Result<bool> {
    crate::log_info!(
        "Running cleanup: operator-sdk cleanup kueue-operator -n openshift-kueue-operator"
    );

    // Run cleanup
    let mut cleanup_cmd = Command::new("operator-sdk");
    if let Some(kc) = kubeconfig {
        cleanup_cmd.env("KUBECONFIG", kc);
    }

    cleanup_cmd.args([
        "cleanup",
        "kueue-operator",
        "-n",
        "openshift-kueue-operator",
    ]);

    let cleanup_output = cleanup_cmd
        .output()
        .context("Failed to run operator-sdk cleanup")?;

    if !cleanup_output.status.success() {
        let cleanup_stderr = String::from_utf8_lossy(&cleanup_output.stderr);
        crate::log_warn!("Cleanup had issues: {}", cleanup_stderr);
    } else {
        crate::log_info!("Cleanup completed successfully");
    }

    // Wait a bit for cleanup to complete
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Retry the bundle installation
    crate::log_info!("Retrying operator-sdk run bundle...");
    let mut retry_cmd = Command::new("operator-sdk");
    if let Some(kc) = kubeconfig {
        retry_cmd.env("KUBECONFIG", kc);
    }

    retry_cmd.args([
        "run",
        "bundle",
        bundle_image,
        "--namespace",
        "openshift-kueue-operator",
        "--timeout",
        "10m",
    ]);

    let retry_output = retry_cmd.output().context("Failed to retry operator-sdk")?;

    if retry_output.status.success() {
        crate::log_info!("Bundle installation successful after cleanup");
        Ok(true)
    } else {
        let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
        let retry_stdout = String::from_utf8_lossy(&retry_output.stdout);
        Err(anyhow::anyhow!(
            "operator-sdk run bundle failed after cleanup:\n{}\n{}",
            retry_stdout,
            retry_stderr
        ))
    }
}

/// Install operator via OLM bundle
pub fn install_bundle(
    bundle_image: &str,
    _cluster_name: &str,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    crate::log_info!("Installing kueue-operator via OLM bundle...");

    // Create namespace
    crate::log_info!("Creating namespace openshift-kueue-operator...");
    let namespace_yaml = r#"apiVersion: v1
kind: Namespace
metadata:
  name: openshift-kueue-operator
"#;
    kubectl::apply_yaml(namespace_yaml, kubeconfig)?;

    // Use operator-sdk run bundle (with retry on catalog exists error)
    crate::log_info!("Running operator-sdk run bundle...");

    let result = run_bundle_with_retry(bundle_image, kubeconfig)?;

    if result {
        crate::log_info!("Operator installed successfully via OLM bundle");
    } else {
        return Err(anyhow::anyhow!(
            "operator-sdk run bundle failed after retry"
        ));
    }

    // Show deployment status
    crate::log_info!("Operator deployment status:");
    kubectl::run_kubectl(
        &["get", "deployments", "-n", "openshift-kueue-operator"],
        kubeconfig,
    )
    .ok();

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_olm_module() {
        // Basic compile test
    }
}
