//! Calico CNI installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install Calico CNI
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing Calico CNI {}...", version);

    let calico_operator_url = format!(
        "https://raw.githubusercontent.com/projectcalico/calico/{}/manifests/tigera-operator.yaml",
        version
    );

    crate::log_info!("Applying Calico operator manifest...");

    // Download and apply the operator manifest
    let operator_yaml = reqwest::blocking::get(&calico_operator_url)
        .context("Failed to download Calico operator manifest")?
        .text()
        .context("Failed to read Calico operator manifest")?;

    kubectl::create_yaml(&operator_yaml, kubeconfig).context("Failed to apply Calico operator")?;

    crate::log_info!("Waiting for Calico CRDs to be established...");

    // Wait for CRDs to be ready
    kubectl::wait_for_condition(
        "crd/installations.operator.tigera.io",
        "condition=established",
        None,
        "60s",
        kubeconfig,
    )
    .context("Failed waiting for Installation CRD")?;

    kubectl::wait_for_condition(
        "crd/apiservers.operator.tigera.io",
        "condition=established",
        None,
        "60s",
        kubeconfig,
    )
    .context("Failed waiting for APIServer CRD")?;

    crate::log_info!("Applying Calico custom resources...");

    // Apply Installation and APIServer resources
    let calico_cr = r#"apiVersion: operator.tigera.io/v1
kind: Installation
metadata:
  name: default
spec:
  calicoNetwork:
    ipPools:
    - blockSize: 26
      cidr: 10.244.0.0/16
      encapsulation: VXLANCrossSubnet
      natOutgoing: Enabled
      nodeSelector: all()
---
apiVersion: operator.tigera.io/v1
kind: APIServer
metadata:
  name: default
spec: {}
"#;

    kubectl::apply_yaml(calico_cr, kubeconfig)
        .context("Failed to apply Calico custom resources")?;

    crate::log_info!("Waiting for Calico pods to be ready...");

    // Wait for tigera-operator pods
    kubectl::wait_for_condition(
        "pod",
        "condition=ready",
        Some("tigera-operator"),
        "300s",
        kubeconfig,
    )
    .ok(); // Ignore errors, continue

    // Wait for calico-system pods (may take longer)
    kubectl::wait_for_condition(
        "pod",
        "condition=ready",
        Some("calico-system"),
        "300s",
        kubeconfig,
    )
    .ok(); // Ignore errors, continue

    // Wait for calico-apiserver pods (may not exist in all deployments)
    kubectl::wait_for_condition(
        "pod",
        "condition=ready",
        Some("calico-apiserver"),
        "60s",
        kubeconfig,
    )
    .ok(); // Ignore errors, it's optional

    crate::log_info!("Calico CNI installed successfully");

    // Wait for nodes to be ready
    crate::log_info!("Waiting for all nodes to be ready...");
    kubectl::wait_for_condition("nodes", "condition=Ready", None, "180s", kubeconfig)
        .context("Nodes did not become ready")?;

    // Display node resources
    crate::log_info!("Cluster node resources:");
    let nodes_output = kubectl::get_nodes(
        "custom-columns=NAME:.metadata.name,CPU:.status.capacity.cpu,MEMORY:.status.capacity.memory",
        kubeconfig,
    ).context("Failed to get node resources")?;

    println!("{}", nodes_output);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_calico_module() {
        // Basic compile test
    }
}
