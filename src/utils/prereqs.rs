//! Prerequisite checking system for required tools

use anyhow::{anyhow, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrereqError {
    #[error("Tool '{name}' not found")]
    NotFound { name: String, hint: String },

    #[error("Failed to check for tool '{name}': {source}")]
    CheckFailed { name: String, source: anyhow::Error },
}

/// Trait for checking prerequisites
pub trait Prerequisite {
    /// Name of the prerequisite tool
    fn name(&self) -> &str;

    /// Check if the tool is available
    fn check(&self) -> Result<(), PrereqError>;

    /// Installation hint for the user
    fn install_hint(&self) -> &str;
}

/// Basic prerequisite that checks if a command exists
pub struct CommandPrereq {
    pub name: String,
    pub hint: String,
}

impl CommandPrereq {
    pub fn new(name: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hint: hint.into(),
        }
    }
}

impl Prerequisite for CommandPrereq {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self) -> Result<(), PrereqError> {
        which::which(&self.name).map_err(|_| PrereqError::NotFound {
            name: self.name.clone(),
            hint: self.hint.clone(),
        })?;
        Ok(())
    }

    fn install_hint(&self) -> &str {
        &self.hint
    }
}

/// Common prerequisites for kueue-dev
pub struct CommonPrereqs;

impl CommonPrereqs {
    /// Get kind prerequisite
    pub fn kind() -> CommandPrereq {
        CommandPrereq::new(
            "kind",
            "Install from: https://kind.sigs.k8s.io/docs/user/quick-start/#installation",
        )
    }

    /// Get kubectl prerequisite
    pub fn kubectl() -> CommandPrereq {
        CommandPrereq::new(
            "kubectl",
            "Install from: https://kubernetes.io/docs/tasks/tools/",
        )
    }

    /// Get go prerequisite
    pub fn go() -> CommandPrereq {
        CommandPrereq::new("go", "Install from: https://golang.org/doc/install")
    }

    /// Get oc (OpenShift CLI) prerequisite
    pub fn oc() -> CommandPrereq {
        CommandPrereq::new(
            "oc",
            "Install from: https://docs.openshift.com/container-platform/latest/cli_reference/openshift_cli/getting-started-cli.html",
        )
    }

    /// Get operator-sdk prerequisite
    pub fn operator_sdk() -> CommandPrereq {
        CommandPrereq::new(
            "operator-sdk",
            "Install from: https://sdk.operatorframework.io/docs/installation/",
        )
    }

    /// Check all prerequisites and return detailed results
    /// Returns (found_tools, missing_tools)
    pub fn check_all(prereqs: &[&dyn Prerequisite]) -> (Vec<String>, Vec<(String, String)>) {
        let mut found = Vec::new();
        let mut missing = Vec::new();

        for prereq in prereqs {
            match prereq.check() {
                Ok(_) => {
                    found.push(prereq.name().to_string());
                }
                Err(e) => match e {
                    PrereqError::NotFound { name, hint } => {
                        missing.push((name, hint));
                    }
                    PrereqError::CheckFailed { name, source } => {
                        crate::log_warn!("Failed to check {}: {}", name, source);
                    }
                },
            }
        }

        (found, missing)
    }
}

/// Check if either docker or podman is available
pub fn check_container_runtime() -> Result<()> {
    let docker_available = which::which("docker").is_ok();
    let podman_available = which::which("podman").is_ok();

    if !docker_available && !podman_available {
        return Err(anyhow!(
            "Neither docker nor podman found. Install one of them:\n  \
             - Docker: https://docs.docker.com/get-docker/\n  \
             - Podman: https://podman.io/getting-started/installation"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prereq_trait() {
        let prereq = CommandPrereq::new("echo", "Should always exist");
        assert_eq!(prereq.name(), "echo");
        assert!(prereq.check().is_ok());
    }

    #[test]
    fn test_missing_prereq() {
        let prereq = CommandPrereq::new("nonexistent-tool-xyz", "Test hint");
        assert!(prereq.check().is_err());
    }
}
