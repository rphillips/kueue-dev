//! Deploy command implementations

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use crate::config::images::ImageConfig;
use crate::config::kueue::{Framework, KueueConfig};
use crate::config::settings::Settings;
use crate::install::{calico, cert_manager, jobset, leaderworkerset, operator};
use crate::k8s::{images, kind, nodes};
use crate::utils::ContainerRuntime;

const CERT_MANAGER_VERSION: &str = "v1.13.3";
const JOBSET_VERSION: &str = "v0.10.1";
const LEADERWORKERSET_VERSION: &str = "v0.7.0";

/// Options for deploying to kind cluster
pub struct DeployKindOptions {
    pub cluster_name: String,
    pub images_file: String,
    pub skip_tests: bool,
    pub skip_kueue_cr: bool,
    pub kueue_frameworks: Option<String>,
    pub kueue_namespace: Option<String>,
}

/// Handle deploy kind command
pub fn deploy_kind(options: DeployKindOptions) -> Result<()> {
    crate::log_info!(
        "Deploying kueue-operator to kind cluster: {}",
        options.cluster_name
    );

    // Get project root
    let project_root = get_project_root()?;

    // Load image configuration
    let images_path = if options.images_file.starts_with('/') {
        PathBuf::from(&options.images_file)
    } else {
        project_root.join(&options.images_file)
    };

    // Always display images configuration (critical deployment info)
    eprintln!();
    eprintln!("Using images from: {}", images_path.display());
    eprintln!();

    let image_config = ImageConfig::load(&images_path)?;

    // Display images that will be used
    eprintln!("Images to be used:");
    eprintln!("  Operator:     {}", image_config.operator()?);
    eprintln!("  Operand:      {}", image_config.operand()?);
    eprintln!("  Must-gather:  {}", image_config.must_gather()?);
    eprintln!();

    // Check if cluster exists
    let cluster = kind::KindCluster::new(&options.cluster_name, kind::CniProvider::Calico);
    if !cluster.exists()? {
        return Err(anyhow::anyhow!(
            "Cluster '{}' does not exist. Create it first with: kueue-dev cluster create --name {}",
            options.cluster_name,
            options.cluster_name
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

    // Canonicalize to get absolute path
    let kubeconfig_path = kubeconfig_path.canonicalize().unwrap_or(kubeconfig_path);

    env::set_var("KUBECONFIG", &kubeconfig_path);
    crate::log_info!("Using kubeconfig: {}", kubeconfig_path.display());

    // Detect container runtime
    let runtime = ContainerRuntime::detect()?;

    // Load images into kind cluster
    images::load_images_to_kind(&options.cluster_name, &image_config, &runtime, true)?;

    // Install cert-manager
    cert_manager::install(CERT_MANAGER_VERSION, Some(&kubeconfig_path))?;

    // Install JobSet
    jobset::install(JOBSET_VERSION, Some(&kubeconfig_path))?;

    // Install LeaderWorkerSet
    leaderworkerset::install(LEADERWORKERSET_VERSION, Some(&kubeconfig_path))?;

    // Install CRDs
    operator::install_crds(&project_root, Some(&kubeconfig_path))?;

    // Build Kueue config if not skipping
    let kueue_config = if options.skip_kueue_cr {
        crate::log_info!("Skipping Kueue CR creation (--skip-kueue-cr flag provided)");
        None
    } else {
        let settings = Settings::load();
        Some(build_kueue_config_from_settings(
            &settings,
            options.kueue_frameworks.as_deref(),
            options.kueue_namespace.as_deref(),
        )?)
    };

    // Install operator with optional Kueue CR
    operator::install_operator_with_config(
        &project_root,
        &image_config,
        kueue_config.as_ref(),
        Some(&kubeconfig_path),
    )?;

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Deployment completed successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Cluster name: {}", options.cluster_name);
    crate::log_info!("Kubeconfig: {}", kubeconfig_path.display());
    crate::log_info!("");
    crate::log_info!("To view operator logs:");
    crate::log_info!(
        "  kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f"
    );
    crate::log_info!("");

    if options.skip_tests {
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
    deploy_kind(DeployKindOptions {
        cluster_name,
        images_file,
        skip_tests,
        skip_kueue_cr: false,
        kueue_frameworks: None,
        kueue_namespace: None,
    })?;

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

/// Build KueueConfig from settings
fn build_kueue_config_from_settings(
    settings: &Settings,
    frameworks_override: Option<&str>,
    namespace_override: Option<&str>,
) -> Result<KueueConfig> {
    let namespace = namespace_override.unwrap_or(&settings.kueue.namespace);

    let mut builder = KueueConfig::builder()
        .name(&settings.kueue.name)
        .namespace(namespace);

    // Use command-line override if provided, otherwise use settings
    let framework_strings: Vec<String> = if let Some(override_str) = frameworks_override {
        override_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        settings.kueue.frameworks.clone()
    };

    // Parse framework strings into Framework enum
    let mut frameworks = Vec::new();
    for fw_str in &framework_strings {
        let framework = match fw_str.as_str() {
            "BatchJob" => Framework::BatchJob,
            "Pod" => Framework::Pod,
            "Deployment" => Framework::Deployment,
            "StatefulSet" => Framework::StatefulSet,
            "JobSet" => Framework::JobSet,
            "LeaderWorkerSet" => Framework::LeaderWorkerSet,
            _ => {
                crate::log_warn!("Unknown framework: {}, skipping", fw_str);
                continue;
            }
        };
        frameworks.push(framework);
    }

    if !frameworks.is_empty() {
        builder = builder.frameworks(frameworks);
    }

    builder.build()
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
