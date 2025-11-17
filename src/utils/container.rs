//! Container runtime detection and operations (Docker/Podman)

use anyhow::{anyhow, Context, Result};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// Detect which container runtime is available
    pub fn detect() -> Result<Self> {
        // Check for docker first
        if which::which("docker").is_ok() {
            crate::log_info!("Using container runtime: docker");
            return Ok(ContainerRuntime::Docker);
        }

        // Fall back to podman
        if which::which("podman").is_ok() {
            crate::log_info!("Using container runtime: podman");
            return Ok(ContainerRuntime::Podman);
        }

        Err(anyhow!(
            "Neither docker nor podman found. Please install one of them:\n  \
             - Docker: https://docs.docker.com/get-docker/\n  \
             - Podman: https://podman.io/getting-started/installation"
        ))
    }

    /// Get the command name for this runtime
    pub fn command(&self) -> &str {
        match self {
            ContainerRuntime::Docker => "docker",
            ContainerRuntime::Podman => "podman",
        }
    }

    /// Check if an image exists locally
    pub fn image_exists(&self, image: &str) -> Result<bool> {
        let output = Command::new(self.command())
            .args(["image", "exists", image])
            .output()
            .with_context(|| format!("Failed to check if image exists: {}", image))?;

        Ok(output.status.success())
    }

    /// Pull an image
    pub fn pull(&self, image: &str) -> Result<()> {
        crate::log_info!("Pulling image: {}", image);

        let status = Command::new(self.command())
            .args(["pull", image])
            .status()
            .with_context(|| format!("Failed to pull image: {}", image))?;

        if !status.success() {
            return Err(anyhow!("Failed to pull image: {}", image));
        }

        Ok(())
    }

    /// Load an image into a kind cluster
    pub fn load_to_kind(&self, image: &str, cluster_name: &str) -> Result<()> {
        crate::log_info!("Loading image into kind cluster: {}", image);

        let mut cmd = Command::new("kind");
        cmd.args(["load", "docker-image", image, "--name", cluster_name]);

        // For podman, set the experimental provider
        if matches!(self, ContainerRuntime::Podman) {
            cmd.env("KIND_EXPERIMENTAL_PROVIDER", "podman");
        }

        let status = cmd
            .status()
            .with_context(|| format!("Failed to load image {} to kind cluster {}", image, cluster_name))?;

        if !status.success() {
            return Err(anyhow!("Failed to load image {} to kind cluster", image));
        }

        Ok(())
    }

    /// Verify an image exists locally, optionally pulling it if needed
    pub fn ensure_image(&self, image: &str, pull_if_missing: bool) -> Result<()> {
        if self.image_exists(image)? {
            crate::log_info!("Image found locally: {}", image);
            return Ok(());
        }

        if pull_if_missing {
            crate::log_warn!("Image not found locally, pulling: {}", image);
            self.pull(image)?;
        } else {
            return Err(anyhow!(
                "Image not found in local registry: {}\nPlease build or pull the image first",
                image
            ));
        }

        Ok(())
    }

    /// Get list of images
    pub fn list_images(&self) -> Result<Vec<String>> {
        let output = Command::new(self.command())
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output()
            .with_context(|| "Failed to list images")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to list images"));
        }

        let images = String::from_utf8(output.stdout)
            .with_context(|| "Failed to parse image list")?
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(images)
    }
}

impl std::fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_runtime() {
        // This test will succeed if at least one runtime is available
        let result = ContainerRuntime::detect();
        // We can't guarantee either is installed, so just test that it returns something sensible
        match result {
            Ok(runtime) => {
                assert!(matches!(runtime, ContainerRuntime::Docker | ContainerRuntime::Podman));
            }
            Err(e) => {
                // If neither is available, error message should mention both
                let msg = e.to_string();
                assert!(msg.contains("docker") || msg.contains("podman"));
            }
        }
    }

    #[test]
    fn test_command_names() {
        assert_eq!(ContainerRuntime::Docker.command(), "docker");
        assert_eq!(ContainerRuntime::Podman.command(), "podman");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ContainerRuntime::Docker), "docker");
        assert_eq!(format!("{}", ContainerRuntime::Podman), "podman");
    }
}
