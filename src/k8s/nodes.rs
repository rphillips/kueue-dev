//! Node management operations

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Label worker nodes with instance-type for e2e tests
pub fn label_worker_nodes(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Labeling worker nodes with instance-type=on-demand...");

    // Get all worker nodes (nodes without the control-plane role)
    let jsonpath = r#"{.items[?(@.metadata.labels.node-role\.kubernetes\.io/control-plane=="")].metadata.name}"#;

    let worker_nodes = kubectl::get_with_jsonpath("nodes", jsonpath, kubeconfig)
        .context("Failed to get worker nodes")?;

    if worker_nodes.trim().is_empty() {
        crate::log_warn!("No worker nodes found to label");
        return Ok(());
    }

    // Split and label each worker node
    for node in worker_nodes.split_whitespace() {
        crate::log_info!("Labeling node: {}", node);
        kubectl::label_node(node, "instance-type=on-demand", kubeconfig)
            .with_context(|| format!("Failed to label node {}", node))?;
    }

    crate::log_info!("Worker nodes labeled successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_module_exists() {
        // Basic compile test
        assert!(true);
    }
}
