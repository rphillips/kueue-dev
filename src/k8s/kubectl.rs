//! Kubectl wrapper utilities

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;

/// Run a kubectl command with optional kubeconfig
pub fn run_kubectl(args: &[&str], kubeconfig: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("kubectl");

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args(args);

    let status = cmd.status().context("Failed to run kubectl command")?;

    if !status.success() {
        return Err(anyhow!("kubectl command failed: {}", args.join(" ")));
    }

    Ok(())
}

/// Run kubectl and capture output
pub fn run_kubectl_output(args: &[&str], kubeconfig: Option<&Path>) -> Result<String> {
    let mut cmd = Command::new("kubectl");

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args(args);

    let output = cmd.output().context("Failed to run kubectl command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "kubectl command failed: {}\n{}",
            args.join(" "),
            stderr
        ));
    }

    Ok(String::from_utf8(output.stdout)?)
}

/// Apply a YAML manifest from string
pub fn apply_yaml(yaml: &str, kubeconfig: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("kubectl");

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args(["apply", "-f", "-"]);

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("Failed to spawn kubectl apply")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(yaml.as_bytes())
            .context("Failed to write YAML to kubectl")?;
    }

    let status = child.wait().context("Failed to wait for kubectl apply")?;

    if !status.success() {
        return Err(anyhow!("kubectl apply failed"));
    }

    Ok(())
}

/// Create a resource from YAML manifest
pub fn create_yaml(yaml: &str, kubeconfig: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("kubectl");

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args(["create", "-f", "-"]);

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("Failed to spawn kubectl create")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(yaml.as_bytes())
            .context("Failed to write YAML to kubectl")?;
    }

    let status = child.wait().context("Failed to wait for kubectl create")?;

    if !status.success() {
        return Err(anyhow!("kubectl create failed"));
    }

    Ok(())
}

/// Wait for a resource to be ready
pub fn wait_for_condition(
    resource: &str,
    condition: &str,
    namespace: Option<&str>,
    timeout: &str,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    let mut args = vec!["wait", "--for", condition, "--timeout", timeout];

    if let Some(ns) = namespace {
        args.push("-n");
        args.push(ns);
    }

    args.push(resource);

    // For resources without a specific name (no slash in resource type), add --all
    // e.g., "pod" or "nodes" but not "pod/my-pod" or "crd/installations.operator.tigera.io"
    if !resource.contains('/') {
        args.push("--all");
    }

    run_kubectl(&args, kubeconfig)
}

/// Get nodes with custom output
pub fn get_nodes(output_format: &str, kubeconfig: Option<&Path>) -> Result<String> {
    run_kubectl_output(&["get", "nodes", "-o", output_format], kubeconfig)
}

/// Label a node
pub fn label_node(node_name: &str, label: &str, kubeconfig: Option<&Path>) -> Result<()> {
    run_kubectl(
        &["label", "nodes", node_name, label, "--overwrite"],
        kubeconfig,
    )
}

/// Get resources with jsonpath
pub fn get_with_jsonpath(
    resource: &str,
    jsonpath: &str,
    kubeconfig: Option<&Path>,
) -> Result<String> {
    run_kubectl_output(
        &["get", resource, "-o", &format!("jsonpath={}", jsonpath)],
        kubeconfig,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kubectl_module_exists() {
        // Basic compile test
        assert!(true);
    }
}
