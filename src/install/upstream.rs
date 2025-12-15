//! Upstream Kueue installation via kustomize or helm

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::k8s::kubectl;

/// Default upstream source path (placeholder - user should set via CLI or config)
pub const DEFAULT_UPSTREAM_SOURCE: &str = "/path/to/kueue/upstream/src";

/// Default kustomize overlay
pub const DEFAULT_OVERLAY: &str = "default";

/// Default helm release name
pub const DEFAULT_RELEASE_NAME: &str = "kueue";

/// Default namespace for upstream kueue
pub const DEFAULT_NAMESPACE: &str = "kueue-system";

/// Options for deploying upstream kueue via kustomize
pub struct KustomizeOptions {
    /// Path to upstream kueue source
    pub source_path: PathBuf,
    /// Kustomize overlay to use (default, dev, alpha-enabled)
    pub overlay: String,
    /// Optional image override for the controller
    pub image: Option<String>,
    /// Namespace to deploy to
    pub namespace: String,
    /// Path to kubeconfig
    pub kubeconfig: Option<PathBuf>,
}

/// Options for deploying upstream kueue via helm
pub struct HelmOptions {
    /// Path to upstream kueue source
    pub source_path: PathBuf,
    /// Helm release name
    pub release_name: String,
    /// Namespace to deploy to
    pub namespace: String,
    /// Optional path to values.yaml override file
    pub values_file: Option<PathBuf>,
    /// Additional --set values
    pub set_values: Vec<String>,
    /// Path to kubeconfig
    pub kubeconfig: Option<PathBuf>,
}

/// Resolve the upstream source path
/// Priority:
/// 1. Explicit path provided via CLI (--source)
/// 2. Path from settings config (defaults.upstream_source)
/// 3. Current working directory (if it looks like a kueue source tree)
pub fn resolve_upstream_source(
    cli_path: Option<&str>,
    settings_path: Option<&str>,
) -> Result<PathBuf> {
    // If explicit CLI path provided, use it
    if let Some(path) = cli_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p.canonicalize().unwrap_or(p));
        }
        return Err(anyhow!("Upstream source path does not exist: {}", path));
    }

    // If settings path provided, use it
    if let Some(path) = settings_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p.canonicalize().unwrap_or(p));
        }
        return Err(anyhow!(
            "Upstream source path from config does not exist: {}",
            path
        ));
    }

    // Try current directory if it looks like a kueue source tree
    let cwd = std::env::current_dir()?;
    if cwd.join("config/default/kustomization.yaml").exists()
        || cwd.join("charts/kueue/Chart.yaml").exists()
    {
        return Ok(cwd);
    }

    Err(anyhow!(
        "No upstream kueue source specified.\n\
         Specify the path with --source or set defaults.upstream_source in config.\n\
         Example: kueue-dev deploy upstream kustomize --source /path/to/kueue/upstream/src"
    ))
}

/// Validate that the upstream source has the expected structure
pub fn validate_upstream_source(source_path: &Path) -> Result<()> {
    // Check for kustomize config
    let kustomize_path = source_path.join("config/default/kustomization.yaml");
    if !kustomize_path.exists() {
        crate::log_warn!(
            "Kustomize config not found at: {}",
            kustomize_path.display()
        );
    }

    // Check for helm chart
    let helm_path = source_path.join("charts/kueue/Chart.yaml");
    if !helm_path.exists() {
        crate::log_warn!("Helm chart not found at: {}", helm_path.display());
    }

    // At least one should exist
    if !kustomize_path.exists() && !helm_path.exists() {
        return Err(anyhow!(
            "Invalid upstream kueue source: neither kustomize config nor helm chart found at {}",
            source_path.display()
        ));
    }

    Ok(())
}

/// Deploy upstream kueue using kustomize
pub fn deploy_kustomize(options: &KustomizeOptions) -> Result<()> {
    crate::log_info!("Deploying upstream kueue via kustomize...");
    crate::log_info!("Source: {}", options.source_path.display());
    crate::log_info!("Overlay: {}", options.overlay);
    crate::log_info!("Namespace: {}", options.namespace);

    // Validate source
    validate_upstream_source(&options.source_path)?;

    // Check kustomize is available
    if which::which("kustomize").is_err() {
        return Err(anyhow!(
            "kustomize is required but not found in PATH.\n\
             Install from: https://kubectl.docs.kubernetes.io/installation/kustomize/"
        ));
    }

    // Build the overlay path
    let overlay_path = options.source_path.join("config").join(&options.overlay);
    if !overlay_path.exists() {
        return Err(anyhow!(
            "Kustomize overlay '{}' not found at: {}",
            options.overlay,
            overlay_path.display()
        ));
    }

    // If image override is specified, we need to use kustomize edit
    if let Some(ref image) = options.image {
        crate::log_info!("Using image override: {}", image);

        // Create a temporary directory and copy the entire config directory
        // This preserves relative path references like ../components/crd
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let config_path = options.source_path.join("config");
        let temp_config = temp_dir.path().join("config");

        // Copy entire config directory to preserve relative paths
        copy_dir_recursive(&config_path, &temp_config)?;

        let temp_overlay = temp_config.join(&options.overlay);

        // Run kustomize edit set image
        let status = Command::new("kustomize")
            .args(["edit", "set", "image", &format!("controller={}", image)])
            .current_dir(&temp_overlay)
            .status()
            .context("Failed to run kustomize edit")?;

        if !status.success() {
            return Err(anyhow!("kustomize edit set image failed"));
        }

        // Build and apply from temp overlay
        apply_kustomize_build(&temp_overlay, options.kubeconfig.as_deref())?;
    } else {
        // Build and apply directly
        apply_kustomize_build(&overlay_path, options.kubeconfig.as_deref())?;
    }

    // Wait for CRDs to be established before the controller starts
    // The workloads.kueue.x-k8s.io CRD is particularly large and may take time to be ready
    crate::log_info!("Waiting for Kueue CRDs to be established...");
    wait_for_kueue_crds(options.kubeconfig.as_deref())?;

    // Wait for deployment to be available
    crate::log_info!("Waiting for kueue-controller-manager deployment...");
    kubectl::wait_for_condition(
        "deployment/kueue-controller-manager",
        "condition=Available",
        Some(&options.namespace),
        "300s",
        options.kubeconfig.as_deref(),
    )
    .context("Kueue controller-manager deployment did not become available")?;

    crate::log_info!("Upstream kueue deployed successfully via kustomize!");

    Ok(())
}

/// Build kustomize output and apply to cluster
fn apply_kustomize_build(overlay_path: &Path, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Building kustomize overlay: {}", overlay_path.display());

    // Run kustomize build
    let kustomize_output = Command::new("kustomize")
        .args(["build", &overlay_path.to_string_lossy()])
        .output()
        .context("Failed to run kustomize build")?;

    if !kustomize_output.status.success() {
        let stderr = String::from_utf8_lossy(&kustomize_output.stderr);
        return Err(anyhow!("kustomize build failed: {}", stderr));
    }

    // Apply the output using server-side apply to avoid annotation size limits
    // Kueue CRDs are large and exceed the 256KB last-applied-configuration annotation limit
    crate::log_info!("Applying kustomize output to cluster (server-side apply)...");

    let mut kubectl_args = vec!["apply", "--server-side", "--force-conflicts", "-f", "-"];
    let kubeconfig_str;
    if let Some(kc) = kubeconfig {
        kubeconfig_str = kc.to_string_lossy().to_string();
        kubectl_args.push("--kubeconfig");
        kubectl_args.push(&kubeconfig_str);
    }

    let mut kubectl_cmd = Command::new("kubectl")
        .args(&kubectl_args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("Failed to spawn kubectl apply")?;

    {
        use std::io::Write;
        let stdin = kubectl_cmd.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(&kustomize_output.stdout)
            .context("Failed to write to kubectl stdin")?;
    }

    let status = kubectl_cmd.wait().context("Failed to wait for kubectl")?;
    if !status.success() {
        return Err(anyhow!("kubectl apply failed"));
    }

    Ok(())
}

/// Deploy upstream kueue using helm
pub fn deploy_helm(options: &HelmOptions) -> Result<()> {
    crate::log_info!("Deploying upstream kueue via helm...");
    crate::log_info!("Source: {}", options.source_path.display());
    crate::log_info!("Release: {}", options.release_name);
    crate::log_info!("Namespace: {}", options.namespace);

    // Validate source
    validate_upstream_source(&options.source_path)?;

    // Check helm is available
    if which::which("helm").is_err() {
        return Err(anyhow!(
            "helm is required but not found in PATH.\n\
             Install from: https://helm.sh/docs/intro/install/"
        ));
    }

    // Build the chart path
    let chart_path = options.source_path.join("charts/kueue");
    if !chart_path.exists() {
        return Err(anyhow!("Helm chart not found at: {}", chart_path.display()));
    }

    // Build helm command
    let mut args = vec![
        "install".to_string(),
        options.release_name.clone(),
        chart_path.to_string_lossy().to_string(),
        "--namespace".to_string(),
        options.namespace.clone(),
        "--create-namespace".to_string(),
    ];

    // Add kubeconfig if specified
    if let Some(ref kc) = options.kubeconfig {
        args.push("--kubeconfig".to_string());
        args.push(kc.to_string_lossy().to_string());
    }

    // Add values file if specified
    if let Some(ref values) = options.values_file {
        if !values.exists() {
            return Err(anyhow!("Values file not found: {}", values.display()));
        }
        args.push("-f".to_string());
        args.push(values.to_string_lossy().to_string());
    }

    // Add --set values
    for set_value in &options.set_values {
        args.push("--set".to_string());
        args.push(set_value.clone());
    }

    crate::log_info!("Running: helm {}", args.join(" "));

    let status = Command::new("helm")
        .args(&args)
        .status()
        .context("Failed to run helm install")?;

    if !status.success() {
        return Err(anyhow!("helm install failed"));
    }

    // Wait for deployment to be available
    crate::log_info!("Waiting for kueue-controller-manager deployment...");
    kubectl::wait_for_condition(
        "deployment/kueue-controller-manager",
        "condition=Available",
        Some(&options.namespace),
        "300s",
        options.kubeconfig.as_deref(),
    )
    .context("Kueue controller-manager deployment did not become available")?;

    crate::log_info!("Upstream kueue deployed successfully via helm!");

    Ok(())
}

/// Default image registry for locally built kueue
pub const DEFAULT_IMAGE_REGISTRY: &str = "localhost";

/// Default image tag for locally built kueue
pub const DEFAULT_IMAGE_TAG: &str = "dev";

/// Build the upstream kueue image using make
/// Returns the full image tag that was built (e.g., localhost/kueue:dev)
///
/// Note: The upstream Makefile uses IMAGE_REGISTRY and appends /kueue to create
/// the full image name. So IMAGE_REGISTRY=localhost results in localhost/kueue:tag
pub fn build_image(source_path: &Path, image_tag: Option<&str>) -> Result<String> {
    crate::log_info!("Source: {}", source_path.display());

    // Validate source has Makefile
    let makefile_path = source_path.join("Makefile");
    if !makefile_path.exists() {
        return Err(anyhow!(
            "Makefile not found at: {}",
            makefile_path.display()
        ));
    }

    // Check for make
    if which::which("make").is_err() {
        return Err(anyhow!(
            "make is required to build images but not found in PATH"
        ));
    }

    // Parse image_tag into registry and tag components
    // If user provides "my-registry/kueue:v1.0", we extract registry="my-registry" and tag="v1.0"
    // If user provides "my-registry:v1.0", we use registry="my-registry" and tag="v1.0"
    // The Makefile will append /kueue to the registry
    let (image_registry, git_tag) = if let Some(tag_str) = image_tag {
        // Check if it has a tag (contains :)
        if let Some(colon_pos) = tag_str.rfind(':') {
            let before_colon = &tag_str[..colon_pos];
            let tag = &tag_str[colon_pos + 1..];

            // If the part before : ends with /kueue, strip it since Makefile adds it
            let registry = before_colon.strip_suffix("/kueue").unwrap_or(before_colon);
            (registry.to_string(), tag.to_string())
        } else {
            // No tag specified, use the whole thing as registry
            let registry = tag_str.strip_suffix("/kueue").unwrap_or(tag_str);
            (registry.to_string(), DEFAULT_IMAGE_TAG.to_string())
        }
    } else {
        (
            DEFAULT_IMAGE_REGISTRY.to_string(),
            DEFAULT_IMAGE_TAG.to_string(),
        )
    };

    // The actual image name will be {registry}/kueue:{tag}
    let full_image = format!("{}/kueue:{}", image_registry, git_tag);
    crate::log_info!("Building upstream kueue image: {}", full_image);

    // Run make kind-image-build with IMAGE_REGISTRY and GIT_TAG
    // This builds and loads the image for use with kind
    crate::log_info!(
        "Running: make kind-image-build IMAGE_REGISTRY={} GIT_TAG={}",
        image_registry,
        git_tag
    );

    let status = Command::new("make")
        .args([
            "kind-image-build",
            &format!("IMAGE_REGISTRY={}", image_registry),
            &format!("GIT_TAG={}", git_tag),
        ])
        .current_dir(source_path)
        .status()
        .context("Failed to run make kind-image-build")?;

    if !status.success() {
        return Err(anyhow!("make kind-image-build failed"));
    }

    crate::log_info!("Image built successfully: {}", full_image);

    Ok(full_image)
}

/// Load a docker image to a kind cluster
pub fn load_image_to_kind(
    cluster_name: &str,
    image: &str,
    runtime: &crate::utils::ContainerRuntime,
) -> Result<()> {
    crate::log_info!("Loading image {} to kind cluster {}", image, cluster_name);

    // Use kind load docker-image or podman equivalent
    let status = Command::new("kind")
        .args(["load", "docker-image", image, "--name", cluster_name])
        .status()
        .context("Failed to run kind load docker-image")?;

    if !status.success() {
        // Try with podman if docker failed
        if matches!(runtime, crate::utils::ContainerRuntime::Podman) {
            crate::log_info!("Retrying with podman save/load...");
            let save_output = Command::new("podman")
                .args(["save", image])
                .output()
                .context("Failed to run podman save")?;

            if !save_output.status.success() {
                return Err(anyhow!("podman save failed"));
            }

            let mut kind_load = Command::new("kind")
                .args([
                    "load",
                    "image-archive",
                    "/dev/stdin",
                    "--name",
                    cluster_name,
                ])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn kind load")?;

            {
                use std::io::Write;
                let stdin = kind_load.stdin.as_mut().expect("Failed to open stdin");
                stdin
                    .write_all(&save_output.stdout)
                    .context("Failed to write to kind stdin")?;
            }

            let status = kind_load.wait().context("Failed to wait for kind load")?;
            if !status.success() {
                return Err(anyhow!("kind load image-archive failed"));
            }
        } else {
            return Err(anyhow!("kind load docker-image failed"));
        }
    }

    crate::log_info!("Image loaded successfully");

    Ok(())
}

/// Build and load kueue image to kind cluster
/// Returns the image tag that was built
pub fn build_and_load_image(
    source_path: &Path,
    cluster_name: &str,
    image_tag: Option<&str>,
    runtime: &crate::utils::ContainerRuntime,
) -> Result<String> {
    // Build the image
    let image = build_image(source_path, image_tag)?;

    // Load to kind cluster
    load_image_to_kind(cluster_name, &image, runtime)?;

    Ok(image)
}

/// Uninstall upstream kueue deployed via helm
pub fn uninstall_helm(
    release_name: &str,
    namespace: &str,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    crate::log_info!(
        "Uninstalling helm release '{}' from namespace '{}'...",
        release_name,
        namespace
    );

    let mut args = vec!["uninstall", release_name, "--namespace", namespace];

    let kubeconfig_str;
    if let Some(kc) = kubeconfig {
        kubeconfig_str = kc.to_string_lossy().to_string();
        args.push("--kubeconfig");
        args.push(&kubeconfig_str);
    }

    let status = Command::new("helm")
        .args(&args)
        .status()
        .context("Failed to run helm uninstall")?;

    if !status.success() {
        crate::log_warn!("helm uninstall returned non-zero exit code");
    }

    Ok(())
}

/// Helper function to recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Wait for all Kueue CRDs to be established
/// This is important because the controller will fail to start if CRDs aren't ready
fn wait_for_kueue_crds(kubeconfig: Option<&Path>) -> Result<()> {
    // Key CRDs that must be established before the controller can start
    let crds = [
        "workloads.kueue.x-k8s.io",
        "clusterqueues.kueue.x-k8s.io",
        "localqueues.kueue.x-k8s.io",
        "resourceflavors.kueue.x-k8s.io",
        "admissionchecks.kueue.x-k8s.io",
    ];

    for crd in &crds {
        crate::log_info!("  Waiting for CRD: {}", crd);
        let resource = format!("crd/{}", crd);
        kubectl::wait_for_condition(&resource, "condition=Established", None, "120s", kubeconfig)
            .with_context(|| format!("CRD {} did not become established", crd))?;
    }

    crate::log_info!("All Kueue CRDs are established");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        assert_eq!(DEFAULT_OVERLAY, "default");
        assert_eq!(DEFAULT_RELEASE_NAME, "kueue");
        assert_eq!(DEFAULT_NAMESPACE, "kueue-system");
    }
}
