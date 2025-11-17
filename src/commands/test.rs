//! Test command implementations

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::images::ImageConfig;
use crate::install::{calico, cert_manager, jobset, leaderworkerset, operator};
use crate::k8s::{images, kind, nodes};
use crate::utils::ContainerRuntime;

const CERT_MANAGER_VERSION: &str = "v1.13.3";
const JOBSET_VERSION: &str = "v0.10.1";
const LEADERWORKERSET_VERSION: &str = "v0.7.0";

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
pub fn run_tests(
    focus: Option<String>,
    kubeconfig: Option<PathBuf>,
) -> Result<()> {
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
    execute_ginkgo_tests(&ginkgo_bin, &project_root, focus, false)?;

    Ok(())
}

/// Run tests with retry loop
pub fn run_tests_with_retry(
    focus: Option<String>,
    kubeconfig: Option<PathBuf>,
) -> Result<()> {
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
        match execute_ginkgo_tests(&ginkgo_bin, &project_root, focus.clone(), false) {
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
pub fn run_tests_kind(
    cluster_name: String,
    focus: Option<String>,
    images_file: String,
) -> Result<()> {
    crate::log_info!("Creating kind cluster and running e2e tests...");

    let project_root = get_project_root()?;

    // Parse CNI provider (always use Calico for tests)
    let cni_provider = kind::CniProvider::Calico;
    let cluster = kind::KindCluster::new(&cluster_name, cni_provider);

    // Create the cluster
    let kubeconfig_path = cluster.create(&project_root)?;

    // Set KUBECONFIG environment variable
    env::set_var("KUBECONFIG", &kubeconfig_path);

    // Install Calico
    calico::install(Some(&kubeconfig_path))?;

    // Label worker nodes
    nodes::label_worker_nodes(Some(&kubeconfig_path))?;

    // Load image configuration
    let images_path = if images_file.starts_with('/') {
        PathBuf::from(images_file)
    } else {
        project_root.join(&images_file)
    };

    let image_config = ImageConfig::load(&images_path)?;

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

    // Run tests with retry
    run_tests_with_retry(focus, Some(kubeconfig_path))?;

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
    std::fs::create_dir_all(&bin_dir)
        .context("Failed to create bin directory")?;

    // Install ginkgo
    let status = Command::new("go")
        .args(&["install", "-mod=mod", "github.com/onsi/ginkgo/v2/ginkgo@v2.1.4"])
        .env("GOBIN", &bin_dir)
        .env("GO111MODULE", "on")
        .status()
        .context("Failed to install ginkgo")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to install ginkgo"));
    }

    if !ginkgo_bin.exists() {
        return Err(anyhow::anyhow!("ginkgo binary not found after installation"));
    }

    crate::log_info!("Ginkgo installed successfully");
    Ok(ginkgo_bin)
}

/// Execute ginkgo tests
fn execute_ginkgo_tests(
    ginkgo_bin: &Path,
    project_root: &Path,
    focus: Option<String>,
    verbose: bool,
) -> Result<()> {
    crate::log_info!("Running e2e tests...");

    let mut args = vec!["--label-filter=!disruptive"];

    if verbose {
        args.push("-v");
    }

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
