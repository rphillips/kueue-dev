//! Test command implementations

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::images::ImageConfig;
use crate::config::kueue::{Framework, KueueConfig};
use crate::config::settings::Settings;
use crate::install::{calico, cert_manager, jobset, leaderworkerset, operator};
use crate::k8s::{images, kind, nodes};
use crate::utils::ContainerRuntime;

const CERT_MANAGER_VERSION: &str = "v1.13.3";
const JOBSET_VERSION: &str = "v0.10.1";
const LEADERWORKERSET_VERSION: &str = "v0.7.0";

/// Options for running tests on kind cluster
pub struct TestKindOptions {
    pub cluster_name: String,
    pub focus: Option<String>,
    pub label_filter: Option<String>,
    pub images_file: String,
    pub skip_kueue_cr: bool,
    pub kueue_frameworks: Option<String>,
    pub kueue_namespace: Option<String>,
}

/// Test skip patterns from test.sh
const TEST_SKIPS: &[&str] = &[
    "AppWrapper",
    "PyTorch",
    "JobSet",
    "LeaderWorkerSet",
    "JAX",
    "Kuberay",
    "Metrics",
    "Fair",
    "TopologyAwareScheduling",
    "Kueue visibility server",
    "Failed Pod can be replaced in group",
    "should allow to schedule a group of diverse pods",
    "StatefulSet created with WorkloadPriorityClass",
];

/// Generate test skip pattern regex
pub fn generate_skip_pattern() -> String {
    format!("({})", TEST_SKIPS.join("|"))
}

/// Run e2e tests on existing cluster
pub fn run_tests(focus: Option<String>, label_filter: Option<String>, kubeconfig: Option<PathBuf>) -> Result<()> {
    let project_root = get_project_root()?;

    // Determine kubeconfig
    let kc = if let Some(path) = kubeconfig {
        path
    } else {
        project_root.join("kube.kubeconfig")
    };

    if !kc.exists() {
        return Err(anyhow::anyhow!(
            "Kubeconfig not found at {}. Please create cluster first.",
            kc.display()
        ));
    }

    env::set_var("KUBECONFIG", &kc);
    crate::log_info!("Using KUBECONFIG: {}", kc.display());

    // Install or check for ginkgo
    let ginkgo_bin = ensure_ginkgo(&project_root)?;

    // Run tests
    execute_ginkgo_tests(&ginkgo_bin, &project_root, focus, label_filter)?;

    Ok(())
}

/// Run tests with retry loop
pub fn run_tests_with_retry(focus: Option<String>, label_filter: Option<String>, kubeconfig: Option<PathBuf>) -> Result<()> {
    let project_root = get_project_root()?;

    // Determine kubeconfig
    let kc = if let Some(path) = kubeconfig {
        path
    } else {
        project_root.join("kube.kubeconfig")
    };

    if !kc.exists() {
        return Err(anyhow::anyhow!(
            "Kubeconfig not found at {}. Please create cluster first.",
            kc.display()
        ));
    }

    env::set_var("KUBECONFIG", &kc);

    // Install or check for ginkgo
    let ginkgo_bin = ensure_ginkgo(&project_root)?;

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Running E2E tests");
    crate::log_info!("==========================================");
    crate::log_info!("");

    // Retry loop
    loop {
        match execute_ginkgo_tests(&ginkgo_bin, &project_root, focus.clone(), label_filter.clone()) {
            Ok(_) => {
                crate::log_info!("");
                crate::log_info!("==========================================");
                crate::log_info!("All tests passed!");
                crate::log_info!("==========================================");
                crate::log_info!("");
                break;
            }
            Err(e) => {
                crate::log_warn!("");
                crate::log_warn!("Tests failed: {}", e);
                crate::log_warn!("You can now debug the cluster.");
                crate::log_warn!("Press RETURN to re-run the tests, or Ctrl+C to exit...");

                crate::utils::wait_for_enter("")?;
                crate::log_info!("Re-running tests...");
            }
        }
    }

    Ok(())
}

/// Create kind cluster and run tests
pub fn run_tests_kind(options: TestKindOptions) -> Result<()> {
    crate::log_info!("Creating kind cluster and running e2e tests...");

    let project_root = get_project_root()?;

    // Parse CNI provider (always use Calico for tests)
    let cni_provider = kind::CniProvider::Calico;
    let cluster = kind::KindCluster::new(&options.cluster_name, cni_provider);

    // Create the cluster
    let kubeconfig_path = cluster.create(&project_root)?;

    // Set KUBECONFIG environment variable
    env::set_var("KUBECONFIG", &kubeconfig_path);

    // Install Calico
    calico::install(Some(&kubeconfig_path))?;

    // Label worker nodes
    nodes::label_worker_nodes(Some(&kubeconfig_path))?;

    // Load image configuration
    let images_path = if options.images_file.starts_with('/') {
        PathBuf::from(&options.images_file)
    } else {
        project_root.join(&options.images_file)
    };

    let image_config = ImageConfig::load(&images_path)?;

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

    // Run tests with retry
    run_tests_with_retry(options.focus, options.label_filter, Some(kubeconfig_path))?;

    Ok(())
}

/// Ensure ginkgo binary is available
fn ensure_ginkgo(project_root: &Path) -> Result<PathBuf> {
    let bin_dir = project_root.join("bin");
    let ginkgo_bin = bin_dir.join("ginkgo");

    if ginkgo_bin.exists() {
        crate::log_info!("Using existing ginkgo at {}", ginkgo_bin.display());
        return Ok(ginkgo_bin);
    }

    crate::log_info!("Installing ginkgo...");

    // Create bin directory
    std::fs::create_dir_all(&bin_dir).context("Failed to create bin directory")?;

    // Install ginkgo
    let status = Command::new("go")
        .args([
            "install",
            "-mod=mod",
            "github.com/onsi/ginkgo/v2/ginkgo@v2.1.4",
        ])
        .env("GOBIN", &bin_dir)
        .env("GO111MODULE", "on")
        .status()
        .context("Failed to install ginkgo")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to install ginkgo"));
    }

    if !ginkgo_bin.exists() {
        return Err(anyhow::anyhow!(
            "ginkgo binary not found after installation"
        ));
    }

    crate::log_info!("Ginkgo installed successfully");
    Ok(ginkgo_bin)
}

/// Execute ginkgo tests
fn execute_ginkgo_tests(
    ginkgo_bin: &Path,
    project_root: &Path,
    focus: Option<String>,
    label_filter: Option<String>,
) -> Result<()> {
    crate::log_info!("Running e2e tests...");

    // Use provided label filter or default to !disruptive
    let label_filter_str = label_filter.as_deref().unwrap_or("!disruptive");
    let label_filter_arg = format!("--label-filter={}", label_filter_str);

    let mut args = vec![label_filter_arg.as_str(), "-v"];

    // Generate skip pattern
    let skip_pattern = generate_skip_pattern();
    args.push("--skip");
    args.push(&skip_pattern);

    // Add focus pattern if provided
    let focus_arg;
    if let Some(ref pattern) = focus {
        crate::log_info!("Running tests with focus: {}", pattern);
        args.push("--focus");
        focus_arg = pattern.clone();
        args.push(&focus_arg);
    }

    // Test directory
    args.push("./test/e2e/...");

    // Run ginkgo
    let status = Command::new(ginkgo_bin)
        .args(&args)
        .current_dir(project_root)
        .status()
        .context("Failed to run ginkgo")?;

    if !status.success() {
        return Err(anyhow::anyhow!("E2E tests failed"));
    }

    crate::log_info!("E2E tests passed successfully!");
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

/// Upstream test skip patterns from e2e-test-ocp.sh
const UPSTREAM_TEST_SKIPS: &[&str] = &[
    // do not deploy AppWrapper in OCP
    "AppWrapper",
    // do not deploy PyTorch in OCP
    "PyTorch",
    // do not deploy JobSet in OCP
    "TrainJob",
    // do not deploy LWS in OCP
    "JAX",
    // do not deploy KubeRay in OCP
    "Kuberay",
    // metrics setup is different than our OCP setup
    "Metrics",
    // ring -> we do not enable Fair sharing by default in our operator
    "Fair",
    // we do not enable this feature in our operator
    "TopologyAwareScheduling",
    // relies on particular CPU setup to force pods to not schedule
    "Failed Pod can be replaced in group",
    // relies on particular CPU setup
    "should allow to schedule a group of diverse pods",
    // relies on particular CPU setup
    "StatefulSet created with WorkloadPriorityClass",
    // We do not have kueuectl in our operator
    "Kueuectl",
];

/// Generate upstream test skip pattern regex
fn generate_upstream_skip_pattern() -> String {
    format!("({})", UPSTREAM_TEST_SKIPS.join("|"))
}

/// Apply git patches to upstream kueue source
fn apply_git_patches(upstream_dir: &Path) -> Result<()> {
    crate::log_info!("Applying git patches to upstream kueue...");

    let patch_dir = upstream_dir.join("patch");
    if !patch_dir.exists() {
        crate::log_warn!("No patch directory found at {}", patch_dir.display());
        return Ok(());
    }

    // Get all .patch files
    let patch_files = std::fs::read_dir(&patch_dir)
        .context("Failed to read patch directory")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                == Some("patch")
        })
        .collect::<Vec<_>>();

    if patch_files.is_empty() {
        crate::log_info!("No patches found in {}", patch_dir.display());
        return Ok(());
    }

    let src_dir = upstream_dir.join("src");
    if !src_dir.exists() {
        return Err(anyhow::anyhow!(
            "Source directory not found at {}",
            src_dir.display()
        ));
    }

    for patch_file in patch_files {
        let patch_path = patch_file.path();

        // Check if patch can be applied (i.e., not already applied)
        let check_status = Command::new("git")
            .args(["apply", "--check", patch_path.to_str().unwrap()])
            .current_dir(&src_dir)
            .output()
            .context("Failed to check git patch")?;

        if check_status.status.success() {
            // Patch can be applied, so apply it
            crate::log_info!("Applying patch: {}", patch_path.display());

            let status = Command::new("git")
                .args(["apply", patch_path.to_str().unwrap()])
                .current_dir(&src_dir)
                .status()
                .context("Failed to apply git patch")?;

            if !status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to apply patch {}",
                    patch_path.display()
                ));
            }
        } else {
            // Patch cannot be applied, likely already applied
            crate::log_info!(
                "Patch {} already applied, skipping",
                patch_path.display()
            );
        }
    }

    crate::log_info!("Git patches applied successfully");
    Ok(())
}

/// Allow privileged access for OpenShift SCC
fn allow_privileged_access(kubeconfig: Option<&PathBuf>) -> Result<()> {
    crate::log_info!("Configuring OpenShift SCC for privileged access...");

    let mut cmd = Command::new("oc");
    cmd.args([
        "adm",
        "policy",
        "add-scc-to-group",
        "privileged",
        "system:authenticated",
        "system:serviceaccounts",
    ]);

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    let status = cmd
        .status()
        .context("Failed to add privileged SCC")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to add privileged SCC"));
    }

    let mut cmd = Command::new("oc");
    cmd.args([
        "adm",
        "policy",
        "add-scc-to-group",
        "anyuid",
        "system:authenticated",
        "system:serviceaccounts",
    ]);

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    let status = cmd
        .status()
        .context("Failed to add anyuid SCC")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to add anyuid SCC"));
    }

    crate::log_info!("OpenShift SCC configured successfully");
    Ok(())
}

/// Run upstream kueue tests
pub fn test_upstream(
    focus: Option<String>,
    label_filter: Option<String>,
    kubeconfig: Option<PathBuf>,
    target: String,
) -> Result<()> {
    crate::log_info!("Running upstream kueue tests...");

    // Get project root
    let project_root = get_project_root()?;

    // Get upstream kueue directory
    let upstream_dir = project_root.join("upstream").join("kueue");
    if !upstream_dir.exists() {
        return Err(anyhow::anyhow!(
            "Upstream kueue directory not found at {}",
            upstream_dir.display()
        ));
    }

    let upstream_src_dir = upstream_dir.join("src");
    if !upstream_src_dir.exists() {
        return Err(anyhow::anyhow!(
            "Upstream kueue src directory not found at {}",
            upstream_src_dir.display()
        ));
    }

    // Apply patches
    apply_git_patches(&upstream_dir)?;

    // Label worker nodes
    crate::log_info!("Labeling worker nodes for e2e tests...");
    nodes::label_worker_nodes(kubeconfig.as_deref())?;

    // Allow privileged access
    allow_privileged_access(kubeconfig.as_ref())?;

    // Ensure ginkgo is available
    let ginkgo_bin = ensure_ginkgo(&upstream_src_dir)?;

    // Build test command
    crate::log_info!("Running upstream e2e tests...");

    let skip_pattern = generate_upstream_skip_pattern();

    let mut args = vec!["--skip", &skip_pattern];

    // Add verbosity
    args.push("-v");

    // Add focus if provided
    let focus_arg;
    if let Some(ref pattern) = focus {
        crate::log_info!("Running tests with focus: {}", pattern);
        args.push("--focus");
        focus_arg = pattern.clone();
        args.push(&focus_arg);
    }

    // Add label filter if provided
    let label_filter_arg;
    if let Some(ref filter) = label_filter {
        crate::log_info!("Running tests with label filter: {}", filter);
        label_filter_arg = format!("--label-filter={}", filter);
        args.push(&label_filter_arg);
    }

    // Add output format
    args.push("--junit-report=junit.xml");
    args.push("--json-report=e2e.json");

    // Add test path
    let test_path = format!("./test/e2e/{}/...", target);
    args.push(&test_path);

    // Set environment variables
    let mut cmd = Command::new(&ginkgo_bin);
    cmd.args(&args)
        .current_dir(&upstream_src_dir)
        .env("KUEUE_NAMESPACE", "openshift-kueue-operator")
        .env("E2E_KIND_VERSION", ""); // Empty for OCP tests

    if let Some(ref kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    let status = cmd.status().context("Failed to run upstream tests")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Upstream e2e tests failed"));
    }

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Upstream e2e tests passed successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_skip_pattern() {
        let pattern = generate_skip_pattern();
        assert!(pattern.contains("AppWrapper"));
        assert!(pattern.contains("PyTorch"));
        assert!(pattern.starts_with('('));
        assert!(pattern.ends_with(')'));
    }

    #[test]
    fn test_test_module() {
        // Basic compile test
        assert!(true);
    }
}
