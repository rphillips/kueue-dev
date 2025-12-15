//! Cluster command implementations

use anyhow::Result;
use std::str::FromStr;

use crate::install::calico;
use crate::k8s::kind::{CniProvider, KindCluster};
use crate::k8s::nodes;

/// Handle cluster create command
pub fn create(name: String, cni: String, kubeconfig: Option<String>) -> Result<()> {
    // Ensure we're in the operator source directory
    crate::utils::ensure_operator_source_directory()?;

    use crate::config::settings::Settings;
    use std::path::PathBuf;

    // Load settings for versions and other config
    let settings = Settings::load();

    crate::log_info!("Creating kind cluster: {}", name);

    let cni_provider = CniProvider::from_str(&cni)?;
    let cluster = KindCluster::new(name, cni_provider);

    // Determine kubeconfig path from CLI arg or config - REQUIRED
    let kubeconfig_path = if let Some(kc) = kubeconfig {
        PathBuf::from(kc)
    } else {
        settings
            .defaults
            .kubeconfig_path
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Kubeconfig path is required. Provide it via --kubeconfig flag or set 'kubeconfig_path' in config file"
                )
            })?
    };

    // Create the cluster with kubeconfig
    let saved_kubeconfig = cluster
        .create_with_kubeconfig(Some(kubeconfig_path))?
        .expect("Kubeconfig should always be saved when path is provided");

    // Install Calico if selected
    if matches!(cni_provider, CniProvider::Calico) {
        calico::install(&settings.versions.calico, Some(&saved_kubeconfig))?;
    } else {
        // Wait for nodes to be ready with default CNI
        crate::log_info!("Waiting for nodes to be ready with default CNI...");
        crate::k8s::kubectl::wait_for_condition(
            "nodes",
            "condition=Ready",
            None,
            "180s",
            Some(&saved_kubeconfig),
        )?;
    }

    // Label worker nodes
    nodes::label_worker_nodes(Some(&saved_kubeconfig))?;

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Cluster created successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Cluster name: {}", cluster.name);
    crate::log_info!("Kubeconfig: {}", saved_kubeconfig.display());
    crate::log_info!("");
    crate::log_info!("To use this cluster, run:");
    crate::log_info!("  export KUBECONFIG={}", saved_kubeconfig.display());
    crate::log_info!("");

    Ok(())
}

/// Handle cluster delete command
pub fn delete(name: String, force: bool) -> Result<()> {
    crate::log_info!("Deleting kind cluster: {}", name);

    let cluster = KindCluster::new(name.clone(), CniProvider::Default);

    if !cluster.exists()? {
        crate::log_warn!("Cluster '{}' does not exist", name);
        return Ok(());
    }

    // Skip confirmation if force flag is set
    if !force
        && !crate::utils::confirm(&format!(
            "Are you sure you want to delete cluster '{}'?",
            name
        ))?
    {
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_cluster_module_exists() {
        // Basic compile test
    }
}
