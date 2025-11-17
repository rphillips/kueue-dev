//! Preflight validation checks before deployment

use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

/// Result of a preflight check
#[derive(Debug, Clone)]
pub enum CheckResult {
    Pass(String),
    Warn(String),
    Fail(String),
}

impl CheckResult {
    pub fn is_error(&self) -> bool {
        matches!(self, CheckResult::Fail(_))
    }

    pub fn is_warning(&self) -> bool {
        matches!(self, CheckResult::Warn(_))
    }

    pub fn display(&self) {
        match self {
            CheckResult::Pass(msg) => {
                println!("  {} {}", "✓".green(), msg);
            }
            CheckResult::Warn(msg) => {
                println!("  {} {}", "⚠".yellow(), msg);
            }
            CheckResult::Fail(msg) => {
                println!("  {} {}", "✗".red(), msg);
            }
        }
    }
}

/// Preflight checker for cluster deployments
pub struct PreflightChecker {
    checks: Vec<CheckResult>,
}

impl PreflightChecker {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Run all preflight checks
    pub fn run_all(&mut self, kubeconfig: Option<&Path>) -> Result<()> {
        crate::log_info!("Running preflight checks...");
        println!();

        self.check_cluster_connection(kubeconfig)?;
        self.check_cluster_version(kubeconfig)?;
        self.check_node_count(kubeconfig)?;
        self.check_existing_installation(kubeconfig)?;

        Ok(())
    }

    /// Display results and return whether deployment should continue
    pub fn display_results(&self) -> bool {
        println!();

        let errors = self.checks.iter().filter(|c| c.is_error()).count();
        let warnings = self.checks.iter().filter(|c| c.is_warning()).count();

        for check in &self.checks {
            check.display();
        }

        println!();

        if errors > 0 {
            println!("{} error(s), {} warning(s)", errors, warnings);
            false
        } else if warnings > 0 {
            println!(
                "{} warning(s). Deployment may continue but proceed with caution.",
                warnings
            );
            true
        } else {
            println!("{}", "All checks passed!".green());
            true
        }
    }

    /// Check if cluster is reachable
    fn check_cluster_connection(&mut self, kubeconfig: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("kubectl");
        if let Some(kc) = kubeconfig {
            cmd.env("KUBECONFIG", kc);
        }

        cmd.args(["cluster-info"]);

        match cmd.output() {
            Ok(output) if output.status.success() => {
                self.checks
                    .push(CheckResult::Pass("Cluster is reachable".to_string()));
            }
            _ => {
                self.checks
                    .push(CheckResult::Fail("Cannot connect to cluster".to_string()));
            }
        }

        Ok(())
    }

    /// Check Kubernetes version
    fn check_cluster_version(&mut self, kubeconfig: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("kubectl");
        if let Some(kc) = kubeconfig {
            cmd.env("KUBECONFIG", kc);
        }

        cmd.args(["version", "--short", "--output=json"]);

        match cmd.output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                if version_str.contains("v1.") {
                    self.checks.push(CheckResult::Pass(format!(
                        "Kubernetes version compatible: {}",
                        version_str.lines().next().unwrap_or("unknown")
                    )));
                } else {
                    self.checks.push(CheckResult::Warn(
                        "Could not determine Kubernetes version".to_string(),
                    ));
                }
            }
            _ => {
                self.checks.push(CheckResult::Warn(
                    "Could not check Kubernetes version".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check node count
    fn check_node_count(&mut self, kubeconfig: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("kubectl");
        if let Some(kc) = kubeconfig {
            cmd.env("KUBECONFIG", kc);
        }

        cmd.args(["get", "nodes", "--no-headers"]);

        match cmd.output() {
            Ok(output) if output.status.success() => {
                let node_count = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count();

                if node_count >= 2 {
                    self.checks.push(CheckResult::Pass(format!(
                        "Cluster has {} nodes (recommended: >= 2)",
                        node_count
                    )));
                } else {
                    self.checks.push(CheckResult::Warn(format!(
                        "Cluster has only {} node(s), recommended: >= 2",
                        node_count
                    )));
                }
            }
            _ => {
                self.checks.push(CheckResult::Warn(
                    "Could not count cluster nodes".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check for existing kueue installation
    fn check_existing_installation(&mut self, kubeconfig: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("kubectl");
        if let Some(kc) = kubeconfig {
            cmd.env("KUBECONFIG", kc);
        }

        cmd.args(["get", "namespace", "openshift-kueue-operator"]);

        match cmd.output() {
            Ok(output) if output.status.success() => {
                self.checks.push(CheckResult::Warn(
                    "Existing kueue installation detected (will be replaced)".to_string(),
                ));
            }
            _ => {
                self.checks.push(CheckResult::Pass(
                    "No existing kueue installation found".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check if specific CRDs exist
    pub fn check_crds(&mut self, crds: &[&str], kubeconfig: Option<&Path>) -> Result<()> {
        for crd in crds {
            let mut cmd = Command::new("kubectl");
            if let Some(kc) = kubeconfig {
                cmd.env("KUBECONFIG", kc);
            }

            cmd.args(["get", "crd", crd]);

            match cmd.output() {
                Ok(output) if output.status.success() => {
                    self.checks
                        .push(CheckResult::Pass(format!("CRD {} exists", crd)));
                }
                _ => {
                    self.checks.push(CheckResult::Warn(format!(
                        "CRD {} not found (will be created)",
                        crd
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check cluster resources
    pub fn check_resources(&mut self, kubeconfig: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("kubectl");
        if let Some(kc) = kubeconfig {
            cmd.env("KUBECONFIG", kc);
        }

        cmd.args(["top", "nodes"]);

        match cmd.output() {
            Ok(output) if output.status.success() => {
                self.checks.push(CheckResult::Pass(
                    "Cluster resource metrics available".to_string(),
                ));
            }
            _ => {
                self.checks.push(CheckResult::Warn(
                    "Could not check cluster resources (metrics-server may not be installed)"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for PreflightChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick preflight check with user confirmation
pub fn run_preflight_with_confirm(kubeconfig: Option<&Path>) -> Result<bool> {
    let mut checker = PreflightChecker::new();
    checker.run_all(kubeconfig)?;

    let can_continue = checker.display_results();

    if !can_continue {
        return Ok(false);
    }

    // If there are warnings, ask for confirmation
    let has_warnings = checker.checks.iter().any(|c| c.is_warning());
    if has_warnings {
        println!();
        return crate::utils::confirm_default_yes("Continue with deployment?");
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_is_error() {
        let pass = CheckResult::Pass("test".to_string());
        let warn = CheckResult::Warn("test".to_string());
        let fail = CheckResult::Fail("test".to_string());

        assert!(!pass.is_error());
        assert!(!warn.is_error());
        assert!(fail.is_error());
    }

    #[test]
    fn test_check_result_is_warning() {
        let pass = CheckResult::Pass("test".to_string());
        let warn = CheckResult::Warn("test".to_string());
        let fail = CheckResult::Fail("test".to_string());

        assert!(!pass.is_warning());
        assert!(warn.is_warning());
        assert!(!fail.is_warning());
    }

    #[test]
    fn test_preflight_checker_new() {
        let checker = PreflightChecker::new();
        assert_eq!(checker.checks.len(), 0);
    }
}
