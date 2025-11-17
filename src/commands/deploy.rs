//! Deploy command implementations

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use crate::config::images::ImageConfig;
use crate::install::{calico, cert_manager, jobset, leaderworkerset, operator};
use crate::k8s::{images, kind, nodes};
use crate::utils::ContainerRuntime;

const CERT_MANAGER_VERSION: &str = "v1.13.3";
const JOBSET_VERSION: &str = "v0.10.1";
const LEADERWORKERSET_VERSION: &str = "v0.7.0";

/// Handle deploy kind command
pub fn deploy_kind(cluster_name: String, images_file: String, skip_tests: bool) -> Result<()> {
    crate::log_info!("Deploying kueue-operator to kind cluster: {}", cluster_name);

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

    // Display images that will be used
    crate::log_info!("");
    crate::log_info!("Images to be used:");
    crate::log_info!("  Operator:     {}", image_config.operator()?);
    crate::log_info!("  Operand:      {}", image_config.operand()?);
    crate::log_info!("  Must-gather:  {}", image_config.must_gather()?);
    crate::log_info!("");

    // Check if cluster exists
    let cluster = kind::KindCluster::new(&cluster_name, kind::CniProvider::Calico);
    if !cluster.exists()? {
        return Err(anyhow::anyhow!(
            "Cluster '{}' does not exist. Create it first with: kueue-dev cluster create --name {}",
            cluster_name,
            cluster_name
        ));
    }

    // Get kubeconfig path
    let kubeconfig_path = project_root.join("kube.kubeconfig");
    if !kubeconfig_path.exists() {
        return Err(anyhow::anyhow!(
            "Kubeconfig not found at {}. Please create cluster first.",
            kubeconfig_path.display()
        ));
    }

    env::set_var("KUBECONFIG", &kubeconfig_path);

    // Detect container runtime
    let runtime = ContainerRuntime::detect()?;

    // Load images into kind cluster
    images::load_images_to_kind(&cluster_name, &image_config, &runtime, true)?;

    // Install cert-manager
    cert_manager::install(CERT_MANAGER_VERSION, Some(&kubeconfig_path))?;

    // Install JobSet
    jobset::install(JOBSET_VERSION, Some(&kubeconfig_path))?;

    // Install LeaderWorkerSet
    leaderworkerset::install(LEADERWORKERSET_VERSION, Some(&kubeconfig_path))?;

    // Install CRDs
    operator::install_crds(&project_root, Some(&kubeconfig_path))?;

    // Install operator
    operator::install_operator(&project_root, &image_config, Some(&kubeconfig_path))?;

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Deployment completed successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Cluster name: {}", cluster_name);
    crate::log_info!("Kubeconfig: {}", kubeconfig_path.display());
    crate::log_info!("");
    crate::log_info!("To view operator logs:");
    crate::log_info!(
        "  kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f"
    );
    crate::log_info!("");

    if skip_tests {
        crate::log_info!("Skipping e2e tests (--skip-tests flag provided)");
    } else {
        crate::log_info!("Note: Test execution will be implemented in Phase 4");
    }

    Ok(())
}

/// Handle deploy kind with full cluster creation and deployment
pub fn deploy_kind_full(
    cluster_name: String,
    images_file: String,
    cni: String,
    skip_tests: bool,
) -> Result<()> {
    crate::log_info!("Creating kind cluster and deploying kueue-operator...");

    // Get project root
    let project_root = get_project_root()?;

    // Parse CNI provider
    let cni_provider = kind::CniProvider::from_str(&cni)?;
    let cluster = kind::KindCluster::new(&cluster_name, cni_provider);

    // Create the cluster
    let kubeconfig_path = cluster.create(&project_root)?;

    // Set KUBECONFIG environment variable
    env::set_var("KUBECONFIG", &kubeconfig_path);

    // Install Calico if selected
    if matches!(cni_provider, kind::CniProvider::Calico) {
        calico::install(Some(&kubeconfig_path))?;
    }

    // Label worker nodes
    nodes::label_worker_nodes(Some(&kubeconfig_path))?;

    // Now deploy the operator
    deploy_kind(cluster_name, images_file, skip_tests)?;

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
    fn test_deploy_module() {
        // Basic compile test
        assert!(true);
    }
}
