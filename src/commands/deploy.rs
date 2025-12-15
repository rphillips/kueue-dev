//! Deploy command implementations

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::str::FromStr;

use crate::config::images::ImageConfig;
use crate::config::kueue::{Framework, KueueConfig};
use crate::config::settings::Settings;
use crate::install::{calico, cert_manager, jobset, leaderworkerset, operator, prometheus};
use crate::k8s::{images, kind, kubectl, nodes};
use crate::utils::ContainerRuntime;

/// Options for deploying to kind cluster
pub struct DeployKindOptions {
    pub cluster_name: String,
    pub images_file: String,
    pub kubeconfig: Option<String>,
    pub skip_tests: bool,
    pub skip_kueue_cr: bool,
    pub kueue_frameworks: Option<String>,
    pub kueue_namespace: Option<String>,
    pub use_bundle: bool,
    /// Version overrides for dependencies
    pub cert_manager_version: Option<String>,
    pub jobset_version: Option<String>,
    pub leaderworkerset_version: Option<String>,
    pub prometheus_version: Option<String>,
}

/// Handle deploy kind command
pub fn deploy_kind(options: DeployKindOptions) -> Result<()> {
    // Ensure we're in the operator source directory
    let source_path = crate::utils::ensure_operator_source_directory()?;

    // Load settings for versions and other config
    let mut settings = Settings::load();

    // Apply version overrides from CLI
    if let Some(ref v) = options.cert_manager_version {
        settings.versions.cert_manager = v.clone();
    }
    if let Some(ref v) = options.jobset_version {
        settings.versions.jobset = v.clone();
    }
    if let Some(ref v) = options.leaderworkerset_version {
        settings.versions.leaderworkerset = v.clone();
    }
    if let Some(ref v) = options.prometheus_version {
        settings.versions.prometheus_operator = v.clone();
    }

    crate::log_info!(
        "Deploying kueue-operator to kind cluster: {}",
        options.cluster_name
    );

    // Load image configuration
    let images_path = PathBuf::from(&options.images_file);

    // Always display images configuration (critical deployment info)
    eprintln!();
    eprintln!("Kueue source path: {}", source_path.display());
    eprintln!("Using images from:  {}", images_path.display());
    eprintln!();

    let image_config = ImageConfig::load(&images_path)?;

    // Display images that will be used
    eprintln!("Images to be used:");
    eprintln!("  Bundle:       {}", image_config.bundle()?);
    eprintln!("  Must-gather:  {}", image_config.must_gather()?);
    eprintln!("  Operator:     {}", image_config.operator()?);
    eprintln!("  Operand:      {}", image_config.operand()?);
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

    // Check for operator-sdk early if using bundle deployment
    if options.use_bundle {
        crate::log_info!("Checking for operator-sdk...");
        if which::which("operator-sdk").is_err() {
            return Err(anyhow::anyhow!(
                "operator-sdk is required for bundle deployment but not found in PATH.\n\
                 Install from: https://sdk.operatorframework.io/docs/installation/\n\
                 Or use --no-bundle to deploy without OLM"
            ));
        }
    }

    // Get kubeconfig path
    let kubeconfig_path = if let Some(ref kc) = options.kubeconfig {
        PathBuf::from(kc)
    } else {
        crate::utils::operator_source_join("kube.kubeconfig")
    };
    if !kubeconfig_path.exists() {
        return Err(anyhow::anyhow!(
            "Kubeconfig not found at {}. Please create cluster first.",
            kubeconfig_path.display()
        ));
    }

    // Canonicalize to get absolute path
    let kubeconfig_path = kubeconfig_path.canonicalize().unwrap_or(kubeconfig_path);
    crate::log_info!("Using kubeconfig: {}", kubeconfig_path.display());

    // Check for and uninstall existing operator installation
    crate::install::olm::uninstall_operator_if_exists(Some(&kubeconfig_path))?;

    // Detect container runtime
    let runtime = ContainerRuntime::detect()?;
    crate::log_info!("Using container runtime: {}", runtime);

    // Delete leader election lease to avoid delays from previous deployments
    // This needs to happen before any operator deployment to ensure quick reconciliation
    crate::log_info!("Cleaning up leader election lease from previous deployments...");
    kubectl::run_kubectl(
        &[
            "delete",
            "lease",
            "openshift-kueue-operator-lock",
            "-n",
            "openshift-kueue-operator",
            "--ignore-not-found",
        ],
        Some(&kubeconfig_path),
    )
    .ok(); // Ignore errors if lease doesn't exist

    if options.use_bundle {
        crate::log_info!("Deploying via OLM bundle...");

        // Start loading images in background while we install dependencies
        crate::log_info!("Starting image load in background...");
        let image_load_handle = images::load_images_to_kind_background(
            options.cluster_name.clone(),
            image_config.clone(),
            runtime,
            true,
        );

        // Install dependencies in parallel while images are loading
        crate::log_info!("Installing dependencies in parallel...");

        let kubeconfig_path_clone1 = kubeconfig_path.clone();
        let kubeconfig_path_clone2 = kubeconfig_path.clone();
        let kubeconfig_path_clone3 = kubeconfig_path.clone();
        let kubeconfig_path_clone4 = kubeconfig_path.clone();
        let kubeconfig_path_clone5 = kubeconfig_path.clone();

        let cert_manager_version = settings.versions.cert_manager.clone();
        let jobset_version = settings.versions.jobset.clone();
        let leaderworkerset_version = settings.versions.leaderworkerset.clone();
        let prometheus_version = settings.versions.prometheus_operator.clone();

        let cert_manager_handle = std::thread::spawn(move || {
            cert_manager::install(&cert_manager_version, Some(&kubeconfig_path_clone1))
        });

        let jobset_handle = std::thread::spawn(move || {
            jobset::install(&jobset_version, Some(&kubeconfig_path_clone2))
        });

        let lws_handle = std::thread::spawn(move || {
            leaderworkerset::install(&leaderworkerset_version, Some(&kubeconfig_path_clone3))
        });

        let olm_handle = std::thread::spawn(move || {
            crate::install::olm::install_olm(Some(&kubeconfig_path_clone4))
        });

        let prometheus_handle = std::thread::spawn(move || {
            prometheus::install(&prometheus_version, Some(&kubeconfig_path_clone5))
        });

        // Wait for all parallel tasks to complete
        cert_manager_handle
            .join()
            .map_err(|e| anyhow::anyhow!("cert-manager thread panicked: {:?}", e))??;
        jobset_handle
            .join()
            .map_err(|e| anyhow::anyhow!("jobset thread panicked: {:?}", e))??;
        lws_handle
            .join()
            .map_err(|e| anyhow::anyhow!("leaderworkerset thread panicked: {:?}", e))??;
        olm_handle
            .join()
            .map_err(|e| anyhow::anyhow!("olm thread panicked: {:?}", e))??;
        prometheus_handle
            .join()
            .map_err(|e| anyhow::anyhow!("prometheus thread panicked: {:?}", e))??;

        // Wait for images to finish loading
        crate::log_info!("Waiting for images to finish loading...");
        image_load_handle
            .join()
            .map_err(|e| anyhow::anyhow!("image load thread panicked: {:?}", e))??;

        // Get bundle image from config
        let bundle_image = image_config.bundle()?;
        crate::log_info!("Using bundle image: {}", bundle_image);

        // Install operator bundle
        crate::install::olm::install_bundle(
            bundle_image,
            &options.cluster_name,
            Some(&kubeconfig_path),
        )?;

        // Wait for operator deployment to be available before creating Kueue CR
        crate::log_info!("Waiting for operator deployment to be available...");
        kubectl::wait_for_condition(
            "deployment/openshift-kueue-operator",
            "condition=Available",
            Some("openshift-kueue-operator"),
            "300s",
            Some(&kubeconfig_path),
        )
        .context("Operator deployment did not become available")?;

        // Give the operator time to start its controllers and be ready to reconcile
        crate::log_info!("Waiting for operator controllers to be ready...");
        std::thread::sleep(std::time::Duration::from_secs(30));

        // Build Kueue config if not skipping
        if !options.skip_kueue_cr {
            let kueue_config = build_kueue_config_from_settings(
                &settings,
                options.kueue_frameworks.as_deref(),
                options.kueue_namespace.as_deref(),
            )?;

            // Create Kueue CR
            operator::create_kueue_cr(&kueue_config, Some(&kubeconfig_path))?;
        } else {
            crate::log_info!("Skipping Kueue CR creation (--skip-kueue-cr flag provided)");
        }
    } else {
        crate::log_info!("Deploying via direct manifests (--no-bundle flag provided)...");

        // Start loading images in background while we install dependencies
        crate::log_info!("Starting image load in background...");
        let image_load_handle = images::load_images_to_kind_background(
            options.cluster_name.clone(),
            image_config.clone(),
            runtime,
            true,
        );

        // Install dependencies in parallel while images are loading
        crate::log_info!("Installing dependencies in parallel...");

        let kubeconfig_path_clone1 = kubeconfig_path.clone();
        let kubeconfig_path_clone2 = kubeconfig_path.clone();
        let kubeconfig_path_clone3 = kubeconfig_path.clone();
        let kubeconfig_path_clone4 = kubeconfig_path.clone();

        let cert_manager_version = settings.versions.cert_manager.clone();
        let jobset_version = settings.versions.jobset.clone();
        let leaderworkerset_version = settings.versions.leaderworkerset.clone();
        let prometheus_version = settings.versions.prometheus_operator.clone();

        let cert_manager_handle = std::thread::spawn(move || {
            cert_manager::install(&cert_manager_version, Some(&kubeconfig_path_clone1))
        });

        let jobset_handle = std::thread::spawn(move || {
            jobset::install(&jobset_version, Some(&kubeconfig_path_clone2))
        });

        let lws_handle = std::thread::spawn(move || {
            leaderworkerset::install(&leaderworkerset_version, Some(&kubeconfig_path_clone3))
        });

        let prometheus_handle = std::thread::spawn(move || {
            prometheus::install(&prometheus_version, Some(&kubeconfig_path_clone4))
        });

        // Wait for all parallel tasks to complete
        cert_manager_handle
            .join()
            .map_err(|e| anyhow::anyhow!("cert-manager thread panicked: {:?}", e))??;
        jobset_handle
            .join()
            .map_err(|e| anyhow::anyhow!("jobset thread panicked: {:?}", e))??;
        lws_handle
            .join()
            .map_err(|e| anyhow::anyhow!("leaderworkerset thread panicked: {:?}", e))??;
        prometheus_handle
            .join()
            .map_err(|e| anyhow::anyhow!("prometheus thread panicked: {:?}", e))??;

        // Wait for images to finish loading
        crate::log_info!("Waiting for images to finish loading...");
        image_load_handle
            .join()
            .map_err(|e| anyhow::anyhow!("image load thread panicked: {:?}", e))??;

        // Install CRDs
        operator::install_crds(Some(&kubeconfig_path))?;

        // Build Kueue config if not skipping
        let kueue_config = if options.skip_kueue_cr {
            crate::log_info!("Skipping Kueue CR creation (--skip-kueue-cr flag provided)");
            None
        } else {
            Some(build_kueue_config_from_settings(
                &settings,
                options.kueue_frameworks.as_deref(),
                options.kueue_namespace.as_deref(),
            )?)
        };

        // Install operator with optional Kueue CR
        operator::install_operator_with_config(
            &image_config,
            kueue_config.as_ref(),
            Some(&kubeconfig_path),
        )?;
    }

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Deployment completed successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Cluster name: {}", options.cluster_name);
    crate::log_info!("Kubeconfig: {}", kubeconfig_path.display());
    crate::log_info!("");

    // Print operator version
    match crate::k8s::kubectl::get_operator_version(Some(&kubeconfig_path)) {
        Ok(version) => {
            crate::log_info!("Operator version: {}", version);
        }
        Err(e) => {
            crate::log_warn!("Could not retrieve operator version: {}", e);
        }
    }

    // Print kueue-controller-manager version if running
    match crate::k8s::kubectl::get_kueue_manager_version(
        "openshift-kueue-operator",
        Some(&kubeconfig_path),
    ) {
        Ok(version) => {
            crate::log_info!("Kueue controller-manager version: {}", version);
        }
        Err(_) => {
            // Don't print warning if kueue-controller-manager is not running
            // as it may not be deployed yet
        }
    }

    crate::log_info!("");
    crate::log_info!("To view operator logs:");
    crate::log_info!(
        "  kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f"
    );
    crate::log_info!("");

    if options.skip_tests {
        crate::log_info!("Skipping e2e tests (--skip-tests flag provided)");
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
    // Ensure we're in the operator source directory
    crate::utils::ensure_operator_source_directory()?;

    crate::log_info!("Creating kind cluster and deploying kueue-operator...");

    // Parse CNI provider
    let cni_provider = kind::CniProvider::from_str(&cni)?;
    let cluster = kind::KindCluster::new(&cluster_name, cni_provider);

    // For deploy_kind_full, we always need to save kubeconfig
    // Use default path if not specified in config
    let settings = crate::config::settings::Settings::load();
    let kubeconfig_to_save = settings
        .defaults
        .kubeconfig_path
        .map(PathBuf::from)
        .or_else(|| Some(crate::utils::operator_source_join("kube.kubeconfig")));

    // Create the cluster
    let kubeconfig_path_opt = cluster.create_with_kubeconfig(kubeconfig_to_save)?;

    // We need a kubeconfig for deployment, so error if not saved
    let kubeconfig_path = kubeconfig_path_opt.ok_or_else(|| {
        anyhow::anyhow!("Kubeconfig was not saved. This should not happen in deploy_kind_full")
    })?;

    // Install Calico if selected
    if matches!(cni_provider, kind::CniProvider::Calico) {
        calico::install(&settings.versions.calico, Some(&kubeconfig_path))?;
    }

    // Label worker nodes
    nodes::label_worker_nodes(Some(&kubeconfig_path))?;

    // Now deploy the operator
    deploy_kind(DeployKindOptions {
        cluster_name,
        images_file,
        kubeconfig: Some(kubeconfig_path.to_string_lossy().to_string()),
        skip_tests,
        skip_kueue_cr: false,
        kueue_frameworks: None,
        kueue_namespace: None,
        use_bundle: true,
        cert_manager_version: None,
        jobset_version: None,
        leaderworkerset_version: None,
        prometheus_version: None,
    })?;

    Ok(())
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
    #[test]
    fn test_deploy_module() {
        // Basic compile test
    }
}
