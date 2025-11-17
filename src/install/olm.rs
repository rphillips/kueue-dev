//! OLM (Operator Lifecycle Manager) installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Install OLM (Operator Lifecycle Manager)
pub fn install_olm(kubeconfig: Option<&Path>) -> Result<()> {
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

/// Install operator via OLM bundle
pub fn install_bundle(
    bundle_image: &str,
    _cluster_name: &str,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    crate::log_info!("Installing kueue-operator via OLM bundle...");
    crate::log_info!("Using bundle image: {}", bundle_image);

    // Create namespace
    crate::log_info!("Creating namespace openshift-kueue-operator...");
    let namespace_yaml = r#"apiVersion: v1
kind: Namespace
metadata:
  name: openshift-kueue-operator
"#;
    kubectl::apply_yaml(namespace_yaml, kubeconfig)?;

    // Use operator-sdk run bundle
    crate::log_info!("Running operator-sdk run bundle...");

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

    let status = cmd.status().context("Failed to run operator-sdk")?;

    if !status.success() {
        return Err(anyhow::anyhow!("operator-sdk run bundle failed"));
    }

    crate::log_info!("Operator installed successfully via OLM bundle");

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
    use super::*;

    #[test]
    fn test_olm_module() {
        // Basic compile test
        assert!(true);
    }
}
