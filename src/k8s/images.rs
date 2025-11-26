//! Container image management and loading

use crate::config::images::ImageConfig;
use crate::utils::ContainerRuntime;
use anyhow::{Context, Result};
use std::thread::{self, JoinHandle};

/// Load images into kind cluster
pub fn load_images_to_kind(
    cluster_name: &str,
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
    pull_if_missing: bool,
) -> Result<()> {
    crate::log_info!("Loading prebuilt images into kind cluster...");

    // Get images from config
    let operator_image = image_config.operator()?;
    let operand_image = image_config.operand()?;
    let must_gather_image = image_config.must_gather()?;
    let bundle_image = image_config.bundle()?;

    // Image used by test workloads (Jobs, Pods, etc)
    let workload_image = std::env::var("CONTAINER_IMAGE")
        .unwrap_or_else(|_| "quay.io/openshift/origin-cli:latest".to_string());

    let images = vec![
        ("operator", operator_image),
        ("operand", operand_image),
        ("must-gather", must_gather_image),
        ("bundle", bundle_image),
        ("workload", workload_image.as_str()),
    ];

    // Verify and pull images if needed
    crate::log_info!(
        "Verifying images exist in local registry{}...",
        if pull_if_missing {
            " (pulling if needed)"
        } else {
            ""
        }
    );

    for (name, image) in &images {
        runtime
            .ensure_image(image, pull_if_missing)
            .with_context(|| format!("Failed to ensure {} image: {}", name, image))?;
    }

    crate::log_info!("All images verified in local registry");

    // Load images into kind cluster
    crate::log_info!("Loading images into kind cluster '{}'...", cluster_name);

    for (name, image) in &images {
        crate::log_info!("Loading {} image: {}", name, image);
        runtime
            .load_to_kind(image, cluster_name)
            .with_context(|| format!("Failed to load {} image to kind", name))?;
    }

    crate::log_info!("All images loaded successfully into kind cluster");
    Ok(())
}

/// Load images into kind cluster in background thread
/// Returns a JoinHandle that can be awaited to ensure images are loaded
pub fn load_images_to_kind_background(
    cluster_name: String,
    image_config: ImageConfig,
    runtime: ContainerRuntime,
    pull_if_missing: bool,
) -> JoinHandle<Result<()>> {
    thread::spawn(move || {
        load_images_to_kind(&cluster_name, &image_config, &runtime, pull_if_missing)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_images_module() {
        // Basic compile test
    }
}
