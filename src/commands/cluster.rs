//! Cluster command implementations

use anyhow::Result;
use std::env;
use std::path::PathBuf;

use crate::k8s::kind::{CniProvider, KindCluster};
use crate::k8s::nodes;
use crate::install::calico;

/// Handle cluster create command
pub fn create(name: String, cni: String) -> Result<()> {
    crate::log_info!("Creating kind cluster: {}", name);

    let cni_provider = CniProvider::from_str(&cni)?;
    let cluster = KindCluster::new(name, cni_provider);

    // Get project root (current directory or parent)
    let project_root = get_project_root()?;

    // Create the cluster
    let kubeconfig_path = cluster.create(&project_root)?;

    // Set KUBECONFIG environment variable for this process
    env::set_var("KUBECONFIG", &kubeconfig_path);

    crate::log_info!("KUBECONFIG set to: {}", kubeconfig_path.display());

    // Install Calico if selected
    if matches!(cni_provider, CniProvider::Calico) {
        calico::install(Some(&kubeconfig_path))?;
    } else {
        // Wait for nodes to be ready with default CNI
        crate::log_info!("Waiting for nodes to be ready with default CNI...");
        crate::k8s::kubectl::wait_for_condition(
            "nodes",
            "condition=Ready",
            None,
            "180s",
            Some(&kubeconfig_path),
        )?;
    }

    // Label worker nodes
    nodes::label_worker_nodes(Some(&kubeconfig_path))?;

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Cluster created successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Cluster name: {}", cluster.name);
    crate::log_info!("Kubeconfig: {}", kubeconfig_path.display());
    crate::log_info!("");
    crate::log_info!("To use this cluster, run:");
    crate::log_info!("  export KUBECONFIG={}", kubeconfig_path.display());
    crate::log_info!("");

    Ok(())
}

/// Handle cluster delete command
pub fn delete(name: String) -> Result<()> {
    crate::log_info!("Deleting kind cluster: {}", name);

    let cluster = KindCluster::new(name.clone(), CniProvider::Default);

    if !cluster.exists()? {
        crate::log_warn!("Cluster '{}' does not exist", name);
        return Ok(());
    }

    if !crate::utils::confirm(&format!("Are you sure you want to delete cluster '{}'?", name))? {
        crate::log_info!("Deletion cancelled");
        return Ok(());
    }

    cluster.delete()?;

    crate::log_info!("");
    crate::log_info!("Cluster '{}' deleted successfully", name);

    Ok(())
}

/// Handle cluster list command
pub fn list() -> Result<()> {
    crate::log_info!("Listing kind clusters...");

    let clusters = KindCluster::list_all()?;

    if clusters.is_empty() {
        crate::log_info!("No kind clusters found");
    } else {
        crate::log_info!("Found {} cluster(s):", clusters.len());
        for cluster in clusters {
            println!("  - {}", cluster);
        }
    }

    Ok(())
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
    fn test_cluster_module_exists() {
        // Basic compile test
        assert!(true);
    }
}
