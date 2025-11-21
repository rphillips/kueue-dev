//! Node management operations

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Label worker nodes with instance-type for e2e tests
/// First worker node gets "instance-type=on-demand"
/// Second worker node gets "instance-type=spot"
pub fn label_worker_nodes(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Labeling worker nodes with instance-type...");

    // Get all worker nodes (nodes without the control-plane role)
    // We can't use JSONPath to check for missing labels, so we'll get all nodes and filter
    let all_nodes = kubectl::run_kubectl_output(&["get", "nodes", "-o", "name"], kubeconfig)
        .context("Failed to get nodes")?;

    // Get control-plane nodes using label selector
    let control_plane_nodes_output = kubectl::run_kubectl_output(
        &[
            "get",
            "nodes",
            "-l",
            "node-role.kubernetes.io/control-plane",
            "-o",
            "name",
        ],
        kubeconfig,
    )
    .unwrap_or_default();

    let control_plane_nodes: std::collections::HashSet<String> = control_plane_nodes_output
        .lines()
        .map(|line| line.trim().strip_prefix("node/").unwrap_or(line.trim()).to_string())
        .collect();

    let mut worker_nodes = Vec::new();
    for node in all_nodes.lines() {
        let node_name = node.trim().strip_prefix("node/").unwrap_or(node.trim());

        // Check if this node is a control-plane node
        if !control_plane_nodes.contains(node_name) {
            worker_nodes.push(node_name.to_string());
        }
    }

    if worker_nodes.is_empty() {
        crate::log_warn!("No worker nodes found to label");
        return Ok(());
    }

    // Sort worker nodes by name for consistent labeling
    worker_nodes.sort();

    for (index, node) in worker_nodes.iter().enumerate() {
        let instance_type = if index == 0 { "on-demand" } else { "spot" };

        crate::log_info!(
            "Labeling node {} with instance-type={}",
            node,
            instance_type
        );
        kubectl::label_node(
            node,
            &format!("instance-type={}", instance_type),
            kubeconfig,
        )
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
