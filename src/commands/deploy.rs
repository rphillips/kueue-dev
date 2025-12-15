//! Deploy command implementations

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::str::FromStr;

use crate::config::images::ImageConfig;
use crate::config::kueue::{Framework, KueueConfig};
use crate::config::settings::Settings;
use crate::install::{
    appwrapper, calico, cert_manager, jobset, leaderworkerset, operator, prometheus,
    training_operator, upstream,
};
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

/// Options for deploying upstream kueue via kustomize
pub struct DeployUpstreamKustomizeOptions {
    /// Path to upstream kueue source (overrides config)
    pub source: Option<String>,
    /// Kustomize overlay to use (default, dev, alpha-enabled)
    pub overlay: String,
    /// Optional image override for the controller
    pub image: Option<String>,
    /// Build kueue image from source
    pub build_image: bool,
    /// Custom image tag when building
    pub image_tag: Option<String>,
    /// Namespace to deploy to
    pub namespace: String,
    /// Path to kubeconfig
    pub kubeconfig: Option<String>,
    /// Cluster name (for kind clusters)
    pub cluster_name: String,
    /// Skip installing dependencies (cert-manager, jobset, leaderworkerset, appwrapper, training-operator)
    pub skip_deps: bool,
    /// Version overrides for dependencies
    pub cert_manager_version: Option<String>,
    pub jobset_version: Option<String>,
    pub leaderworkerset_version: Option<String>,
    pub appwrapper_version: Option<String>,
    pub training_operator_version: Option<String>,
}

/// Options for deploying upstream kueue via helm
pub struct DeployUpstreamHelmOptions {
    /// Path to upstream kueue source (overrides config)
    pub source: Option<String>,
    /// Helm release name
    pub release_name: String,
    /// Namespace to deploy to
    pub namespace: String,
    /// Optional path to values.yaml override file
    pub values_file: Option<String>,
    /// Additional --set values
    pub set_values: Vec<String>,
    /// Build kueue image from source
    pub build_image: bool,
    /// Custom image tag when building
    pub image_tag: Option<String>,
    /// Path to kubeconfig
    pub kubeconfig: Option<String>,
    /// Cluster name (for kind clusters)
    pub cluster_name: String,
    /// Skip installing dependencies (cert-manager, jobset, leaderworkerset, appwrapper, training-operator)
    pub skip_deps: bool,
    /// Version overrides for dependencies
    pub cert_manager_version: Option<String>,
    pub jobset_version: Option<String>,
    pub leaderworkerset_version: Option<String>,
    pub appwrapper_version: Option<String>,
    pub training_operator_version: Option<String>,
}

/// Deploy upstream kueue via kustomize
pub fn deploy_upstream_kustomize(options: DeployUpstreamKustomizeOptions) -> Result<()> {
    let settings = Settings::load();

    // Resolve upstream source path
    let source_path = upstream::resolve_upstream_source(
        options.source.as_deref(),
        settings.defaults.upstream_source.as_deref(),
    )?;

    crate::log_info!("Deploying upstream kueue via kustomize");
    crate::log_info!("Source: {}", source_path.display());
    crate::log_info!("Overlay: {}", options.overlay);
    crate::log_info!("Namespace: {}", options.namespace);

    // Get kubeconfig path
    let kubeconfig_path = resolve_kubeconfig(&options.kubeconfig, &options.cluster_name)?;
    crate::log_info!("Using kubeconfig: {}", kubeconfig_path.display());

    // Build image if requested
    let image = if options.build_image {
        let runtime = ContainerRuntime::detect()?;
        crate::log_info!("Building and loading kueue image...");
        let built_image = upstream::build_and_load_image(
            &source_path,
            &options.cluster_name,
            options.image_tag.as_deref(),
            &runtime,
        )?;
        Some(built_image)
    } else {
        options.image
    };

    // Install dependencies if not skipped
    if !options.skip_deps {
        install_upstream_dependencies(
            &kubeconfig_path,
            &settings,
            options.cert_manager_version.as_deref(),
            options.jobset_version.as_deref(),
            options.leaderworkerset_version.as_deref(),
            options.appwrapper_version.as_deref(),
            options.training_operator_version.as_deref(),
        )?;
    } else {
        crate::log_info!("Skipping dependency installation (--skip-deps)");
    }

    // Deploy via kustomize
    let kustomize_options = upstream::KustomizeOptions {
        source_path,
        overlay: options.overlay,
        image,
        namespace: options.namespace.clone(),
        kubeconfig: Some(kubeconfig_path.clone()),
    };

    upstream::deploy_kustomize(&kustomize_options)?;

    print_upstream_success(&options.cluster_name, &kubeconfig_path, &options.namespace);

    Ok(())
}

/// Deploy upstream kueue via helm
pub fn deploy_upstream_helm(options: DeployUpstreamHelmOptions) -> Result<()> {
    let settings = Settings::load();

    // Resolve upstream source path
    let source_path = upstream::resolve_upstream_source(
        options.source.as_deref(),
        settings.defaults.upstream_source.as_deref(),
    )?;

    crate::log_info!("Deploying upstream kueue via helm");
    crate::log_info!("Source: {}", source_path.display());
    crate::log_info!("Release: {}", options.release_name);
    crate::log_info!("Namespace: {}", options.namespace);

    // Get kubeconfig path
    let kubeconfig_path = resolve_kubeconfig(&options.kubeconfig, &options.cluster_name)?;
    crate::log_info!("Using kubeconfig: {}", kubeconfig_path.display());

    // Build image if requested and add to set_values
    let mut set_values = options.set_values;
    if options.build_image {
        let runtime = ContainerRuntime::detect()?;
        crate::log_info!("Building and loading kueue image...");
        let built_image = upstream::build_and_load_image(
            &source_path,
            &options.cluster_name,
            options.image_tag.as_deref(),
            &runtime,
        )?;
        // Add image override to helm values
        // Parse image into repository and tag
        if let Some(pos) = built_image.rfind(':') {
            let repo = &built_image[..pos];
            let tag = &built_image[pos + 1..];
            set_values.push(format!(
                "controllerManager.manager.image.repository={}",
                repo
            ));
            set_values.push(format!("controllerManager.manager.image.tag={}", tag));
        } else {
            set_values.push(format!(
                "controllerManager.manager.image.repository={}",
                built_image
            ));
        }
        // Set imagePullPolicy to Never for locally loaded images
        set_values.push("controllerManager.manager.image.pullPolicy=Never".to_string());
    }

    // Install dependencies if not skipped
    if !options.skip_deps {
        install_upstream_dependencies(
            &kubeconfig_path,
            &settings,
            options.cert_manager_version.as_deref(),
            options.jobset_version.as_deref(),
            options.leaderworkerset_version.as_deref(),
            options.appwrapper_version.as_deref(),
            options.training_operator_version.as_deref(),
        )?;
    } else {
        crate::log_info!("Skipping dependency installation (--skip-deps)");
    }

    // Deploy via helm
    let helm_options = upstream::HelmOptions {
        source_path,
        release_name: options.release_name,
        namespace: options.namespace.clone(),
        values_file: options.values_file.map(PathBuf::from),
        set_values,
        kubeconfig: Some(kubeconfig_path.clone()),
    };

    upstream::deploy_helm(&helm_options)?;

    print_upstream_success(&options.cluster_name, &kubeconfig_path, &options.namespace);

    Ok(())
}

/// Resolve kubeconfig path from options or cluster name
fn resolve_kubeconfig(kubeconfig: &Option<String>, cluster_name: &str) -> Result<PathBuf> {
    if let Some(kc) = kubeconfig {
        let path = PathBuf::from(kc);
        if path.exists() {
            return Ok(path.canonicalize().unwrap_or(path));
        }
        return Err(anyhow::anyhow!("Kubeconfig not found: {}", kc));
    }

    // Try default locations
    let default_path = crate::utils::operator_source_join("kube.kubeconfig");
    if default_path.exists() {
        return Ok(default_path.canonicalize().unwrap_or(default_path));
    }

    // Try kind cluster kubeconfig
    let home_dir = std::env::var("HOME").unwrap_or_default();
    let kind_kubeconfig = PathBuf::from(format!("{}/.kube/config-{}", home_dir, cluster_name));
    if kind_kubeconfig.exists() {
        return Ok(kind_kubeconfig);
    }

    // Try default kubeconfig
    let default_kubeconfig = PathBuf::from(format!("{}/.kube/config", home_dir));
    if default_kubeconfig.exists() {
        return Ok(default_kubeconfig);
    }

    Err(anyhow::anyhow!(
        "Could not find kubeconfig. Specify with --kubeconfig or ensure cluster exists."
    ))
}

/// Install dependencies for upstream kueue (cert-manager, jobset, leaderworkerset, appwrapper, training-operator)
#[allow(clippy::too_many_arguments)]
fn install_upstream_dependencies(
    kubeconfig: &std::path::Path,
    settings: &Settings,
    cert_manager_version: Option<&str>,
    jobset_version: Option<&str>,
    leaderworkerset_version: Option<&str>,
    appwrapper_version: Option<&str>,
    training_operator_version: Option<&str>,
) -> Result<()> {
    crate::log_info!("Installing dependencies in parallel...");

    let kubeconfig_clone1 = kubeconfig.to_path_buf();
    let kubeconfig_clone2 = kubeconfig.to_path_buf();
    let kubeconfig_clone3 = kubeconfig.to_path_buf();
    let kubeconfig_clone4 = kubeconfig.to_path_buf();
    let kubeconfig_clone5 = kubeconfig.to_path_buf();

    let cert_manager_ver = cert_manager_version
        .map(String::from)
        .unwrap_or_else(|| settings.versions.cert_manager.clone());
    let jobset_ver = jobset_version
        .map(String::from)
        .unwrap_or_else(|| settings.versions.jobset.clone());
    let leaderworkerset_ver = leaderworkerset_version
        .map(String::from)
        .unwrap_or_else(|| settings.versions.leaderworkerset.clone());
    let appwrapper_ver = appwrapper_version
        .map(String::from)
        .unwrap_or_else(|| settings.versions.appwrapper.clone());
    let training_operator_ver = training_operator_version
        .map(String::from)
        .unwrap_or_else(|| settings.versions.training_operator.clone());

    let cert_manager_handle = std::thread::spawn(move || {
        cert_manager::install(&cert_manager_ver, Some(&kubeconfig_clone1))
    });

    let jobset_handle =
        std::thread::spawn(move || jobset::install(&jobset_ver, Some(&kubeconfig_clone2)));

    let lws_handle = std::thread::spawn(move || {
        leaderworkerset::install(&leaderworkerset_ver, Some(&kubeconfig_clone3))
    });

    let appwrapper_handle =
        std::thread::spawn(move || appwrapper::install(&appwrapper_ver, Some(&kubeconfig_clone4)));

    let training_operator_handle = std::thread::spawn(move || {
        training_operator::install(&training_operator_ver, Some(&kubeconfig_clone5))
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
    appwrapper_handle
        .join()
        .map_err(|e| anyhow::anyhow!("appwrapper thread panicked: {:?}", e))??;
    training_operator_handle
        .join()
        .map_err(|e| anyhow::anyhow!("training-operator thread panicked: {:?}", e))??;

    crate::log_info!("Dependencies installed successfully");

    Ok(())
}

/// Print success message for upstream deployment
fn print_upstream_success(cluster_name: &str, kubeconfig: &std::path::Path, namespace: &str) {
    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Upstream kueue deployment completed!");
    crate::log_info!("==========================================");
    crate::log_info!("");
    crate::log_info!("Cluster name: {}", cluster_name);
    crate::log_info!("Kubeconfig: {}", kubeconfig.display());
    crate::log_info!("Namespace: {}", namespace);
    crate::log_info!("");
    crate::log_info!("To view kueue logs:");
    crate::log_info!(
        "  kubectl logs -n {} -l control-plane=controller-manager -f",
        namespace
    );
    crate::log_info!("");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_deploy_module() {
        // Basic compile test
    }
}
