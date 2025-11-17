//! Kueue-dev CLI - Development tool for kueue-operator

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use kueue_dev::utils::{CommonPrereqs, ContainerRuntime, Prerequisite};
use kueue_dev::{log_error, log_info};
use std::io;

#[derive(Parser)]
#[command(name = "kueue-dev")]
#[command(author, version, about = "Development CLI tool for kueue-operator", long_about = None)]
struct Cli {
    /// Verbose output (can be used multiple times: -v, -vv, -vvv)
    /// -v: INFO, -vv: DEBUG, -vvv: TRACE
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Dry-run mode: show what would be done without making changes
    #[arg(long, global = true)]
    dry_run: bool,

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
    Check {
        /// Check for kind cluster tools
        #[arg(long)]
        kind: bool,

        /// Check for OpenShift tools
        #[arg(long)]
        openshift: bool,

        /// Check for OLM tools
        #[arg(long)]
        olm: bool,
    },

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

        /// CNI provider (calico or default)
        #[arg(long, default_value = "calico")]
        cni: String,
    },

    /// Delete a kind cluster
    Delete {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,
    },

    /// List kind clusters
    List,
}

#[derive(Subcommand)]
enum DeployCommands {
    /// Deploy to kind cluster with prebuilt images
    Kind {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Path to related images JSON file
        #[arg(long, default_value = "related_images.rphillips.json")]
        images: String,

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
        #[arg(long, default_value = "related_images.rphillips.json")]
        images: String,

        /// Skip tests after deployment
        #[arg(long)]
        skip_tests: bool,
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

    /// Create kind cluster and run tests
    Kind {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Test focus pattern
        #[arg(short, long)]
        focus: Option<String>,

        /// Label filter for tests (e.g., "!disruptive", "network-policy")
        #[arg(short = 'l', long)]
        label_filter: Option<String>,

        /// Path to related images JSON file
        #[arg(long, default_value = "related_images.rphillips.json")]
        images: String,

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

    /// Run tests on OpenShift cluster
    Openshift {
        /// Test focus pattern
        #[arg(short, long)]
        focus: Option<String>,
    },
}

#[derive(Subcommand)]
enum ImagesCommands {
    /// List images from config
    List {
        /// Path to related images JSON file
        #[arg(short, long, default_value = "related_images.rphillips.json")]
        file: String,
    },

    /// Load images to kind cluster
    Load {
        /// Cluster name
        #[arg(short, long, default_value = "kueue-test")]
        name: String,

        /// Path to related images JSON file
        #[arg(long, default_value = "related_images.rphillips.json")]
        images: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity level
    let log_level = match cli.verbose {
        0 => "warn",  // Default: only warnings and errors
        1 => "info",  // -v: info level
        2 => "debug", // -vv: debug level
        _ => "trace", // -vvv: trace level
    };
    std::env::set_var("RUST_LOG", log_level);

    // Set dry-run mode
    if cli.dry_run {
        std::env::set_var("KUEUE_DEV_DRY_RUN", "1");
        crate::log_info!("🔍 DRY RUN MODE: No changes will be made");
        println!();
    }

    match cli.command {
        Commands::Cluster { command } => handle_cluster_command(command),
        Commands::Deploy { command } => handle_deploy_command(command),
        Commands::Test { command } => handle_test_command(command),
        Commands::Cleanup { kubeconfig } => handle_cleanup_command(kubeconfig),
        Commands::Images { command } => handle_images_command(command),
        Commands::Check {
            kind,
            openshift,
            olm,
        } => handle_check_command(kind, openshift, olm),
        Commands::Interactive { kubeconfig } => handle_interactive_command(kubeconfig),
        Commands::Completion { shell } => handle_completion_command(shell),
        Commands::Version => handle_version_command(),
    }
}

fn handle_cluster_command(command: ClusterCommands) -> Result<()> {
    match command {
        ClusterCommands::Create { name, cni } => kueue_dev::commands::cluster::create(name, cni),
        ClusterCommands::Delete { name } => kueue_dev::commands::cluster::delete(name),
        ClusterCommands::List => kueue_dev::commands::cluster::list(),
    }
}

fn handle_deploy_command(command: DeployCommands) -> Result<()> {
    match command {
        DeployCommands::Kind {
            name,
            images,
            skip_tests,
            skip_kueue_cr,
            kueue_frameworks,
            kueue_namespace,
        } => {
            use kueue_dev::commands::deploy::DeployKindOptions;
            kueue_dev::commands::deploy::deploy_kind(DeployKindOptions {
                cluster_name: name,
                images_file: images,
                skip_tests,
                skip_kueue_cr,
                kueue_frameworks,
                kueue_namespace,
            })
        }
        DeployCommands::Olm { bundle, name } => {
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
            log_info!("  kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f --kubeconfig={}", kubeconfig.display());
            log_info!("");

            Ok(())
        }
        DeployCommands::Openshift { images, skip_tests } => {
            kueue_dev::commands::openshift::deploy_openshift(images, skip_tests)
        }
    }
}

fn handle_test_command(command: TestCommands) -> Result<()> {
    use std::path::PathBuf;

    match command {
        TestCommands::Run { focus, label_filter, kubeconfig } => {
            let kc = kubeconfig.map(PathBuf::from);
            kueue_dev::commands::test::run_tests_with_retry(focus, label_filter, kc)
        }
        TestCommands::Kind {
            name,
            focus,
            label_filter,
            images,
            skip_kueue_cr,
            kueue_frameworks,
            kueue_namespace,
        } => {
            use kueue_dev::commands::test::TestKindOptions;
            kueue_dev::commands::test::run_tests_kind(TestKindOptions {
                cluster_name: name,
                focus,
                label_filter,
                images_file: images,
                skip_kueue_cr,
                kueue_frameworks,
                kueue_namespace,
            })
        }
        TestCommands::Openshift { focus } => {
            // For OpenShift, we expect the user to be logged in with oc
            // The tests will use the current context
            kueue_dev::commands::test::run_tests_with_retry(focus, None, None)
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
            use kueue_dev::k8s::images::load_images_to_kind;
            use kueue_dev::utils::ContainerRuntime;

            let path = PathBuf::from(&images);
            let config = ImageConfig::load(&path)?;
            let runtime = ContainerRuntime::detect()?;

            load_images_to_kind(&name, &config, &runtime, true)
        }
    }
}

fn handle_check_command(kind: bool, openshift: bool, olm: bool) -> Result<()> {
    log_info!("Checking prerequisites...");

    // Create owned prerequisite objects
    let kubectl = CommonPrereqs::kubectl();
    let kind_prereq = CommonPrereqs::kind();
    let go = CommonPrereqs::go();
    let oc = CommonPrereqs::oc();
    let operator_sdk = CommonPrereqs::operator_sdk();

    // Build vector of references
    let mut prereqs: Vec<&dyn Prerequisite> = vec![&kubectl];

    if kind {
        prereqs.push(&kind_prereq);
        prereqs.push(&go);
    }

    if openshift {
        prereqs.push(&oc);
    }

    if olm {
        prereqs.push(&operator_sdk);
    }

    // Check container runtime
    let runtime = ContainerRuntime::detect()?;
    log_info!("Container runtime: {}", runtime);

    // Check all prerequisites
    match CommonPrereqs::check_all(&prereqs) {
        Ok(_) => {
            log_info!("✓ All prerequisites satisfied!");
            Ok(())
        }
        Err(e) => {
            log_error!("{}", e);
            std::process::exit(1);
        }
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
    println!("kueue-dev {}", env!("CARGO_PKG_VERSION"));
    println!("Development CLI tool for kueue-operator");
    Ok(())
}
