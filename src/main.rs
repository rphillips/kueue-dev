//! Kueue-dev CLI - Development tool for kueue-operator

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use kueue_dev::config::settings::Settings;
use kueue_dev::utils::{CommonPrereqs, ContainerRuntime, Prerequisite};
use kueue_dev::{log_error, log_info, log_warn};
use std::io;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn get_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        let version_str = match built_info::GIT_VERSION {
            Some(git_ver) => {
                let clean_git_ver = git_ver.strip_prefix('v').unwrap_or(git_ver);
                clean_git_ver.to_string()
            }
            None => match built_info::GIT_COMMIT_HASH_SHORT {
                Some(hash) => format!("{}+{}", built_info::PKG_VERSION, hash),
                None => built_info::PKG_VERSION.to_string(),
            },
        };
        let rustc_ver = built_info::RUSTC_VERSION
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown");
        format!("{} (rustc {})", version_str, rustc_ver)
    })
}

#[derive(Parser)]
#[command(name = "kueue-dev")]
#[command(author, version = get_version(), about = "Development CLI tool for kueue-operator", long_about = None)]
struct Cli {
    /// Verbose output (can be used multiple times: -v, -vv, -vvv)
    /// -v: INFO, -vv: DEBUG, -vvv: TRACE
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to kueue-operator source directory
    #[arg(
        short = 's',
        long = "source",
        global = true,
        env = "KUEUE_OPERATOR_SOURCE"
    )]
    operator_source: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage kind clusters
    Cluster {
        #[command(subcommand)]
        command: ClusterCommands,
    },

    /// Deploy kueue-operator
    Deploy {
        #[command(subcommand)]
        command: DeployCommands,
    },

    /// Run e2e tests
    Test {
        #[command(subcommand)]
        command: TestCommands,
    },

    /// Clean up test resources
    Cleanup {
        /// Path to kubeconfig file
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,
    },

    /// Manage container images
    Images {
        #[command(subcommand)]
        command: ImagesCommands,
    },

    /// Check prerequisites
    Check,

    /// Interactive debugging menu
    Interactive {
        /// Path to kubeconfig file
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum ClusterCommands {
    /// Create a new kind cluster
    Create {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// CNI provider (default or calico)
        #[arg(long)]
        cni: Option<String>,

        /// Path to save kubeconfig file (if not specified, kubeconfig won't be saved)
        #[arg(short, long)]
        kubeconfig: Option<String>,
    },

    /// Delete a kind cluster
    Delete {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List kind clusters
    List,
}

#[derive(Subcommand)]
enum DeployCommands {
    /// Deploy kueue-operator (OpenShift operator)
    Operator {
        #[command(subcommand)]
        command: DeployOperatorCommands,
    },

    /// Deploy upstream kueue (vanilla Kubernetes)
    Upstream {
        #[command(subcommand)]
        command: DeployUpstreamCommands,
    },
}

#[derive(Subcommand)]
enum DeployOperatorCommands {
    /// Deploy to kind cluster with prebuilt images
    Kind {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Path to related images JSON file
        #[arg(long = "related-images")]
        images: Option<String>,

        /// Path to kubeconfig file
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,

        /// Skip tests after deployment
        #[arg(long)]
        skip_tests: bool,

        /// Skip creating Kueue CR (only deploy operator)
        #[arg(long)]
        skip_kueue_cr: bool,

        /// Kueue frameworks to enable (comma-separated)
        /// Valid values: BatchJob, Pod, Deployment, StatefulSet, JobSet, LeaderWorkerSet
        #[arg(long)]
        kueue_frameworks: Option<String>,

        /// Kueue CR namespace (default: openshift-kueue-operator)
        #[arg(long)]
        kueue_namespace: Option<String>,

        /// Deploy without OLM bundle (use direct manifest deployment)
        #[arg(long)]
        no_bundle: bool,

        /// Override cert-manager version (e.g., v1.18.0)
        #[arg(long)]
        cert_manager_version: Option<String>,

        /// Override JobSet version (e.g., v0.10.1)
        #[arg(long)]
        jobset_version: Option<String>,

        /// Override LeaderWorkerSet version (e.g., v0.7.0)
        #[arg(long)]
        leaderworkerset_version: Option<String>,

        /// Override Prometheus Operator version (e.g., v0.82.2)
        #[arg(long)]
        prometheus_version: Option<String>,
    },

    /// Deploy via OLM bundle
    Olm {
        /// Bundle image
        #[arg(short, long)]
        bundle: String,

        /// Cluster name
        #[arg(short = 'n', long, default_value = "kueue-test")]
        name: String,
    },

    /// Deploy to OpenShift cluster
    Openshift {
        /// Path to related images JSON file
        #[arg(long = "related-images")]
        images: Option<String>,

        /// Skip tests after deployment
        #[arg(long)]
        skip_tests: bool,
    },
}

#[derive(Subcommand)]
enum DeployUpstreamCommands {
    /// Deploy using kustomize
    Kustomize {
        /// Path to upstream kueue source directory
        #[arg(long = "upstream-source", env = "KUEUE_UPSTREAM_SOURCE")]
        source: Option<String>,

        /// Kustomize overlay to use (default, dev, alpha-enabled)
        #[arg(short, long, default_value = "default")]
        overlay: String,

        /// Override controller image (ignored if --build-image is used)
        #[arg(long)]
        image: Option<String>,

        /// Build kueue image from source and load to kind cluster
        #[arg(long)]
        build_image: bool,

        /// Custom image tag when building (default: localhost/kueue:dev)
        #[arg(long)]
        image_tag: Option<String>,

        /// Namespace to deploy to
        #[arg(short, long, default_value = "kueue-system")]
        namespace: String,

        /// Cluster name (for kind clusters)
        #[arg(short = 'c', long, default_value = "kueue-test")]
        cluster_name: String,

        /// Path to kubeconfig file
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,

        /// Skip installing dependencies (cert-manager, jobset, leaderworkerset)
        #[arg(long)]
        skip_deps: bool,

        /// Override cert-manager version (e.g., v1.18.0)
        #[arg(long)]
        cert_manager_version: Option<String>,

        /// Override JobSet version (e.g., v0.10.1)
        #[arg(long)]
        jobset_version: Option<String>,

        /// Override LeaderWorkerSet version (e.g., v0.7.0)
        #[arg(long)]
        leaderworkerset_version: Option<String>,

        /// Override AppWrapper version (e.g., v1.1.2)
        #[arg(long)]
        appwrapper_version: Option<String>,

        /// Override Kubeflow Training Operator version (e.g., v1.8.1)
        #[arg(long)]
        training_operator_version: Option<String>,
    },

    /// Deploy using helm
    Helm {
        /// Path to upstream kueue source directory
        #[arg(long = "upstream-source", env = "KUEUE_UPSTREAM_SOURCE")]
        source: Option<String>,

        /// Helm release name
        #[arg(short, long, default_value = "kueue")]
        release_name: String,

        /// Namespace to deploy to
        #[arg(short, long, default_value = "kueue-system")]
        namespace: String,

        /// Path to values.yaml override file
        #[arg(short = 'f', long)]
        values_file: Option<String>,

        /// Set helm values (can be repeated, e.g., --set key=value)
        #[arg(long = "set", value_name = "KEY=VALUE")]
        set_values: Vec<String>,

        /// Build kueue image from source and load to kind cluster
        #[arg(long)]
        build_image: bool,

        /// Custom image tag when building (default: localhost/kueue:dev)
        #[arg(long)]
        image_tag: Option<String>,

        /// Cluster name (for kind clusters)
        #[arg(short = 'c', long, default_value = "kueue-test")]
        cluster_name: String,

        /// Path to kubeconfig file
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,

        /// Skip installing dependencies (cert-manager, jobset, leaderworkerset)
        #[arg(long)]
        skip_deps: bool,

        /// Override cert-manager version (e.g., v1.18.0)
        #[arg(long)]
        cert_manager_version: Option<String>,

        /// Override JobSet version (e.g., v0.10.1)
        #[arg(long)]
        jobset_version: Option<String>,

        /// Override LeaderWorkerSet version (e.g., v0.7.0)
        #[arg(long)]
        leaderworkerset_version: Option<String>,

        /// Override AppWrapper version (e.g., v1.1.2)
        #[arg(long)]
        appwrapper_version: Option<String>,

        /// Override Kubeflow Training Operator version (e.g., v1.8.1)
        #[arg(long)]
        training_operator_version: Option<String>,
    },
}

#[derive(Subcommand)]
enum TestCommands {
    /// Run tests on existing cluster
    Run {
        /// Test focus pattern
        #[arg(short, long)]
        focus: Option<String>,

        /// Label filter for tests (e.g., "!disruptive", "network-policy")
        #[arg(short = 'l', long)]
        label_filter: Option<String>,

        /// Path to kubeconfig
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,
    },

    /// Deploy operator and run tests
    Operator {
        /// Type of cluster (kind, openshift, or kubeconfig)
        #[arg(short = 't', long, value_parser = ["kind", "openshift", "kubeconfig"], default_value = "kubeconfig")]
        r#type: String,

        /// Cluster name (kind only)
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Test focus pattern
        #[arg(short, long)]
        focus: Option<String>,

        /// Label filter for tests (e.g., "!disruptive", "network-policy")
        #[arg(short = 'l', long)]
        label_filter: Option<String>,

        /// Path to kubeconfig (kubeconfig type only)
        #[arg(short = 'k', long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,

        /// Path to related images JSON file (kind only)
        #[arg(long = "related-images")]
        images: Option<String>,

        /// Skip creating Kueue CR (only deploy operator)
        #[arg(long)]
        skip_kueue_cr: bool,

        /// Kueue frameworks to enable (comma-separated)
        /// Valid values: BatchJob, Pod, Deployment, StatefulSet, JobSet, LeaderWorkerSet
        #[arg(long)]
        kueue_frameworks: Option<String>,

        /// Kueue CR namespace (default: openshift-kueue-operator)
        #[arg(long)]
        kueue_namespace: Option<String>,
    },

    /// Run upstream kueue tests (requires OpenShift cluster)
    Upstream {
        /// Test focus pattern
        #[arg(short, long)]
        focus: Option<String>,

        /// Label filter for tests (e.g., "!disruptive", "network-policy")
        #[arg(short = 'l', long)]
        label_filter: Option<String>,

        /// Path to kubeconfig
        #[arg(short, long, env = "KUBECONFIG")]
        kubeconfig: Option<String>,

        /// E2E target folder (default: singlecluster)
        #[arg(long, default_value = "singlecluster")]
        target: String,
    },
}

#[derive(Subcommand)]
enum ImagesCommands {
    /// Build and push container images
    Build {
        /// Components to build (operator, operand, must-gather). Defaults to all components if not specified.
        #[arg(value_delimiter = ',')]
        components: Vec<String>,

        /// Path to images configuration file (defaults to config file setting)
        #[arg(short, long = "related-images")]
        images: Option<String>,

        /// Build components in parallel
        #[arg(short, long)]
        parallel: bool,
    },

    /// List images from config
    List {
        /// Path to related images JSON file
        #[arg(short, long, default_value = "related_images.json")]
        file: String,
    },

    /// Load images to kind cluster
    Load {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Path to related images JSON file
        #[arg(long = "related-images")]
        images: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity level
    let log_level = match cli.verbose {
        0 => "info",  // Default: info level
        1 => "debug", // -v: debug level
        2 => "trace", // -vv: trace level
        _ => "trace", // -vvv: trace level
    };

    // Initialize tracing subscriber with custom formatting
    // Format matches the old style: [LEVEL] message
    // Use EnvFilter::try_new to set the log level without modifying environment variables
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(false)
                .with_level(true)
                .with_ansi(true)
                .without_time()
                .with_writer(std::io::stderr),
        )
        .with(env_filter)
        .init();

    // Set the operator source path from CLI if provided
    kueue_dev::utils::set_cli_operator_source(cli.operator_source);

    match cli.command {
        Commands::Cluster { command } => handle_cluster_command(command),
        Commands::Deploy { command } => handle_deploy_command(command),
        Commands::Test { command } => handle_test_command(command),
        Commands::Cleanup { kubeconfig } => handle_cleanup_command(kubeconfig),
        Commands::Images { command } => handle_images_command(command),
        Commands::Check => handle_check_command(),
        Commands::Interactive { kubeconfig } => handle_interactive_command(kubeconfig),
        Commands::Completion { shell } => handle_completion_command(shell),
        Commands::Version => handle_version_command(),
    }
}

fn handle_cluster_command(command: ClusterCommands) -> Result<()> {
    match command {
        ClusterCommands::Create {
            name,
            cni,
            kubeconfig,
        } => {
            let settings = Settings::load();
            let cni = cni.unwrap_or(settings.defaults.cni_provider);
            kueue_dev::commands::cluster::create(name, cni, kubeconfig)
        }
        ClusterCommands::Delete { name, force } => {
            kueue_dev::commands::cluster::delete(name, force)
        }
        ClusterCommands::List => kueue_dev::commands::cluster::list(),
    }
}

fn handle_deploy_command(command: DeployCommands) -> Result<()> {
    match command {
        DeployCommands::Operator { command } => handle_deploy_operator_command(command),
        DeployCommands::Upstream { command } => handle_deploy_upstream_command(command),
    }
}

fn handle_deploy_operator_command(command: DeployOperatorCommands) -> Result<()> {
    match command {
        DeployOperatorCommands::Kind {
            name,
            images,
            kubeconfig,
            skip_tests,
            skip_kueue_cr,
            kueue_frameworks,
            kueue_namespace,
            no_bundle,
            cert_manager_version,
            jobset_version,
            leaderworkerset_version,
            prometheus_version,
        } => {
            use kueue_dev::commands::deploy::DeployKindOptions;
            use kueue_dev::config::settings::Settings;

            // Use provided images file or fall back to config file setting
            let settings = Settings::load();
            let images_file = images.unwrap_or(settings.defaults.images_file);

            kueue_dev::commands::deploy::deploy_kind(DeployKindOptions {
                cluster_name: name,
                images_file,
                kubeconfig,
                skip_tests,
                skip_kueue_cr,
                kueue_frameworks,
                kueue_namespace,
                use_bundle: !no_bundle,
                cert_manager_version,
                jobset_version,
                leaderworkerset_version,
                prometheus_version,
            })
        }
        DeployOperatorCommands::Olm { bundle, name } => {
            use kueue_dev::install::olm;
            use std::env;
            use std::path::PathBuf;

            log_info!("Deploying via OLM to cluster: {}", name);
            log_info!("Bundle image: {}", bundle);

            // Get kubeconfig for kind cluster
            let home_dir = env::var("HOME").expect("HOME environment variable not set");
            let kubeconfig = PathBuf::from(format!("{}/.kube/config-{}", home_dir, name));

            if !kubeconfig.exists() {
                log_error!("Kubeconfig not found: {}", kubeconfig.display());
                log_error!("Cluster {} may not exist. Create it first with:", name);
                log_error!("  kueue-dev cluster create --name {}", name);
                std::process::exit(1);
            }

            // Install OLM
            olm::install_olm(Some(&kubeconfig))?;

            // Install operator bundle
            olm::install_bundle(&bundle, &name, Some(&kubeconfig))?;

            log_info!("");
            log_info!("==========================================");
            log_info!("OLM deployment completed successfully!");
            log_info!("==========================================");
            log_info!("");
            log_info!("To view operator logs:");
            log_info!(
                "  kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f --kubeconfig={}",
                kubeconfig.display()
            );
            log_info!("");

            Ok(())
        }
        DeployOperatorCommands::Openshift { images, skip_tests } => {
            use kueue_dev::config::settings::Settings;

            // Use provided images file or fall back to config file setting
            let settings = Settings::load();
            let images_file = images.unwrap_or(settings.defaults.images_file);

            kueue_dev::commands::openshift::deploy_openshift(images_file, skip_tests)
        }
    }
}

fn handle_deploy_upstream_command(command: DeployUpstreamCommands) -> Result<()> {
    match command {
        DeployUpstreamCommands::Kustomize {
            source,
            overlay,
            image,
            build_image,
            image_tag,
            namespace,
            cluster_name,
            kubeconfig,
            skip_deps,
            cert_manager_version,
            jobset_version,
            leaderworkerset_version,
            appwrapper_version,
            training_operator_version,
        } => {
            use kueue_dev::commands::deploy::DeployUpstreamKustomizeOptions;

            kueue_dev::commands::deploy::deploy_upstream_kustomize(DeployUpstreamKustomizeOptions {
                source,
                overlay,
                image,
                build_image,
                image_tag,
                namespace,
                kubeconfig,
                cluster_name,
                skip_deps,
                cert_manager_version,
                jobset_version,
                leaderworkerset_version,
                appwrapper_version,
                training_operator_version,
            })
        }
        DeployUpstreamCommands::Helm {
            source,
            release_name,
            namespace,
            values_file,
            set_values,
            build_image,
            image_tag,
            cluster_name,
            kubeconfig,
            skip_deps,
            cert_manager_version,
            jobset_version,
            leaderworkerset_version,
            appwrapper_version,
            training_operator_version,
        } => {
            use kueue_dev::commands::deploy::DeployUpstreamHelmOptions;

            kueue_dev::commands::deploy::deploy_upstream_helm(DeployUpstreamHelmOptions {
                source,
                release_name,
                namespace,
                values_file,
                set_values,
                build_image,
                image_tag,
                kubeconfig,
                cluster_name,
                skip_deps,
                cert_manager_version,
                jobset_version,
                leaderworkerset_version,
                appwrapper_version,
                training_operator_version,
            })
        }
    }
}

fn handle_test_command(command: TestCommands) -> Result<()> {
    use std::path::PathBuf;

    match command {
        TestCommands::Run {
            focus,
            label_filter,
            kubeconfig,
        } => {
            let kc = kubeconfig.map(PathBuf::from);
            kueue_dev::commands::test::run_tests_with_retry(focus, label_filter, kc)
        }
        TestCommands::Operator {
            r#type,
            name,
            focus,
            label_filter,
            kubeconfig,
            images,
            skip_kueue_cr,
            kueue_frameworks,
            kueue_namespace,
        } => {
            use kueue_dev::config::settings::Settings;

            match r#type.as_str() {
                "kind" => {
                    use kueue_dev::commands::test::TestKindOptions;

                    // Use provided images file or fall back to config file setting
                    let settings = Settings::load();
                    let images_file = images.unwrap_or(settings.defaults.images_file);

                    kueue_dev::commands::test::run_tests_kind(TestKindOptions {
                        cluster_name: name,
                        focus,
                        label_filter,
                        images_file,
                        skip_kueue_cr,
                        kueue_frameworks,
                        kueue_namespace,
                    })
                }
                "openshift" => {
                    // For OpenShift, we expect the user to be logged in with oc
                    // The tests will use the current context
                    kueue_dev::commands::test::run_tests_with_retry(focus, label_filter, None)
                }
                "kubeconfig" => {
                    // Use the provided or environment kubeconfig
                    let kc = kubeconfig.map(PathBuf::from);
                    kueue_dev::commands::test::run_tests_with_retry(focus, label_filter, kc)
                }
                _ => Err(anyhow::anyhow!("Invalid operator type: {}", r#type)),
            }
        }
        TestCommands::Upstream {
            focus,
            label_filter,
            kubeconfig,
            target,
        } => {
            let kc = kubeconfig.map(PathBuf::from);
            kueue_dev::commands::test::test_upstream(focus, label_filter, kc, target)
        }
    }
}

fn handle_cleanup_command(kubeconfig: Option<String>) -> Result<()> {
    use std::path::PathBuf;

    let kc = kubeconfig.as_ref().map(PathBuf::from);
    kueue_dev::commands::cleanup::cleanup(kc.as_deref())
}

fn handle_images_command(command: ImagesCommands) -> Result<()> {
    use kueue_dev::config::images::ImageConfig;
    use std::path::PathBuf;

    match command {
        ImagesCommands::Build {
            components,
            images,
            parallel,
        } => kueue_dev::commands::build::build_and_push(components, images, parallel),
        ImagesCommands::List { file } => {
            let path = PathBuf::from(&file);
            let config = ImageConfig::load(&path)?;

            log_info!("Images from: {}", file);
            println!();
            for (name, image) in config.list() {
                println!("  {}: {}", name, image);
            }
            Ok(())
        }
        ImagesCommands::Load { name, images } => {
            use kueue_dev::config::settings::Settings;
            use kueue_dev::k8s::images::load_images_to_kind;
            use kueue_dev::utils::ContainerRuntime;

            // Use provided images file or fall back to config file setting
            let settings = Settings::load();
            let images_file = images.unwrap_or(settings.defaults.images_file);

            let path = PathBuf::from(&images_file);
            let config = ImageConfig::load(&path)?;
            let runtime = ContainerRuntime::detect()?;
            log_info!("Using container runtime: {}", runtime);

            load_images_to_kind(&name, &config, &runtime, true)
        }
    }
}

fn handle_check_command() -> Result<()> {
    log_info!("Checking all prerequisites...");
    log_info!("");

    // Create owned prerequisite objects
    let kubectl = CommonPrereqs::kubectl();
    let kind_prereq = CommonPrereqs::kind();
    let go = CommonPrereqs::go();
    let oc = CommonPrereqs::oc();
    let operator_sdk = CommonPrereqs::operator_sdk();
    let kustomize = CommonPrereqs::kustomize();
    let helm = CommonPrereqs::helm();

    // Build vector of all prerequisites
    let prereqs: Vec<&dyn Prerequisite> = vec![&kubectl, &kind_prereq, &go, &oc, &operator_sdk];

    // Optional prerequisites for upstream deployment
    let optional_prereqs: Vec<&dyn Prerequisite> = vec![&kustomize, &helm];

    // Check container runtime
    let container_runtime_available = match ContainerRuntime::detect() {
        Ok(runtime) => {
            log_info!("✓ Container runtime: {}", runtime);
            true
        }
        Err(_) => {
            log_error!("✗ Container runtime: Neither docker nor podman found");
            false
        }
    };

    log_info!("");

    // Check all prerequisites
    let (found, missing) = CommonPrereqs::check_all(&prereqs);

    // Check optional prerequisites
    let (optional_found, optional_missing) = CommonPrereqs::check_all(&optional_prereqs);

    // Display found tools
    if !found.is_empty() {
        log_info!("Required tools:");
        for tool in &found {
            log_info!("  ✓ {}", tool);
        }
        log_info!("");
    }

    // Display missing tools
    if !missing.is_empty() {
        log_error!("Missing required tools:");
        for (name, hint) in &missing {
            log_error!("  ✗ {} - {}", name, hint);
        }
        log_info!("");
    }

    // Display optional tools (for upstream deployment)
    log_info!("Optional tools (for upstream deployment):");
    for tool in &optional_found {
        log_info!("  ✓ {}", tool);
    }
    for (name, _hint) in &optional_missing {
        log_warn!("  - {} (not installed)", name);
    }
    log_info!("");

    // Summary
    log_info!("==========================================");
    log_info!("Summary:");
    log_info!(
        "  Required: {}/{}",
        found.len(),
        found.len() + missing.len()
    );
    log_info!(
        "  Optional: {}/{}",
        optional_found.len(),
        optional_found.len() + optional_missing.len()
    );

    if !container_runtime_available {
        log_info!("  Container runtime: Missing");
    } else {
        log_info!("  Container runtime: OK");
    }

    log_info!("==========================================");
    log_info!("");

    // Exit with error if required tools are missing
    if !missing.is_empty() || !container_runtime_available {
        log_error!(
            "Some required prerequisites are missing. Please install them before proceeding."
        );
        std::process::exit(1);
    } else {
        log_info!("✓ All required prerequisites satisfied!");
        if !optional_missing.is_empty() {
            log_info!(
                "  (Optional tools missing: install kustomize/helm for 'deploy upstream' commands)"
            );
        }
        Ok(())
    }
}

fn handle_interactive_command(kubeconfig: Option<String>) -> Result<()> {
    use std::path::PathBuf;

    let kc = kubeconfig.as_ref().map(PathBuf::from);
    kueue_dev::commands::interactive::show_menu(kc.as_deref())
}

fn handle_completion_command(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "kueue-dev", &mut io::stdout());
    Ok(())
}

fn handle_version_command() -> Result<()> {
    println!("kueue-dev {}", get_version());
    Ok(())
}
