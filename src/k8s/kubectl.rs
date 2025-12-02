//! Kubectl wrapper utilities

use anyhow::{Context, Result, anyhow};
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

/// Apply a YAML manifest using server-side apply (for large CRDs)
pub fn apply_yaml_server_side(yaml: &str, kubeconfig: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("kubectl");

    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args(["apply", "--server-side", "-f", "-"]);

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("Failed to spawn kubectl apply --server-side")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(yaml.as_bytes())
            .context("Failed to write YAML to kubectl")?;
    }

    let status = child
        .wait()
        .context("Failed to wait for kubectl apply --server-side")?;

    if !status.success() {
        return Err(anyhow!("kubectl apply --server-side failed"));
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

/// Get operator version from pod logs
pub fn get_operator_version(kubeconfig: Option<&Path>) -> Result<String> {
    // Get the pod name
    let pod_name = run_kubectl_output(
        &[
            "get",
            "pods",
            "-n",
            "openshift-kueue-operator",
            "-l",
            "name=openshift-kueue-operator",
            "-o",
            "jsonpath={.items[0].metadata.name}",
        ],
        kubeconfig,
    )?;

    if pod_name.is_empty() {
        return Err(anyhow!("No operator pod found"));
    }

    // Get first 10 lines of logs
    let logs = run_kubectl_output(
        &[
            "logs",
            &pod_name,
            "-n",
            "openshift-kueue-operator",
            "--tail=10",
        ],
        kubeconfig,
    )?;

    // Look for version in logs (common patterns: "version", "Version", "v=")
    for line in logs.lines() {
        if let Some(version) = extract_version_from_log(line) {
            return Ok(version);
        }
    }

    Err(anyhow!("Version not found in operator logs"))
}

/// Get kueue-controller-manager version from pod logs
pub fn get_kueue_manager_version(namespace: &str, kubeconfig: Option<&Path>) -> Result<String> {
    // Get the pod name
    let pod_name = run_kubectl_output(
        &[
            "get",
            "pods",
            "-n",
            namespace,
            "-l",
            "control-plane=controller-manager",
            "-o",
            "jsonpath={.items[0].metadata.name}",
        ],
        kubeconfig,
    )?;

    if pod_name.is_empty() {
        return Err(anyhow!("No kueue-controller-manager pod found"));
    }

    // Get first 10 lines of logs
    let logs = run_kubectl_output(
        &["logs", &pod_name, "-n", namespace, "--tail=10"],
        kubeconfig,
    )?;

    // Look for version in logs
    for line in logs.lines() {
        if let Some(version) = extract_version_from_log(line) {
            return Ok(version);
        }
    }

    Err(anyhow!(
        "Version not found in kueue-controller-manager logs"
    ))
}

/// Extract version from a log line
fn extract_version_from_log(line: &str) -> Option<String> {
    // Look for common version patterns in logs
    // Examples:
    // - openshift-kueue-operator version v0.0.0-unknown-78aa1392-78aa1392
    // - "gitVersion":"v0.15.0-rc.0-51-g8e20b4c71-dirty"
    // - "version": "v1.2.3"

    // Check for gitVersion in JSON format (kueue-controller-manager)
    if let Some(pos) = line.find("\"gitVersion\"") {
        let after = &line[pos..];
        if let Some(start) = after.find(':') {
            let value_start = &after[start + 1..].trim_start();
            if let Some(quote_start) = value_start.find('"') {
                if let Some(quote_end) = value_start[quote_start + 1..].find('"') {
                    return Some(
                        value_start[quote_start + 1..quote_start + 1 + quote_end].to_string(),
                    );
                }
            }
        }
    }

    // Check for "openshift-kueue-operator version X" format
    if let Some(pos) = line.find("openshift-kueue-operator version") {
        let after = &line[pos + "openshift-kueue-operator version".len()..].trim();
        if let Some(end) = after.find(|c: char| c.is_whitespace()) {
            return Some(after[..end].to_string());
        } else if !after.is_empty() {
            return Some(after.to_string());
        }
    }

    // Generic version extraction
    if let Some(pos) = line.to_lowercase().find("version") {
        let after_version = &line[pos..];
        // Try to extract quoted version
        if let Some(start) = after_version.find('"') {
            if let Some(end) = after_version[start + 1..].find('"') {
                return Some(after_version[start + 1..start + 1 + end].to_string());
            }
        }
        // Try to extract version after colon or equals
        if let Some(start) = after_version.find(':').or_else(|| after_version.find('=')) {
            let version_part = after_version[start + 1..].trim();
            if let Some(end) = version_part.find(|c: char| c.is_whitespace() || c == ',') {
                return Some(version_part[..end].to_string());
            } else if !version_part.is_empty() {
                return Some(version_part.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kubectl_module_exists() {
        // Basic compile test
    }

    #[test]
    fn test_extract_operator_version() {
        let log_line = "I1120 21:25:34.555797       1 builder.go:304] openshift-kueue-operator version v0.0.0-unknown-78aa1392-78aa1392";
        let version = extract_version_from_log(log_line);
        assert_eq!(
            version,
            Some("v0.0.0-unknown-78aa1392-78aa1392".to_string())
        );
    }

    #[test]
    fn test_extract_kueue_version_json() {
        let log_line = r#"{"level":"info","ts":"2025-11-20T21:26:00.770553599Z","logger":"setup","caller":"kueue/main.go:155","msg":"Initializing","gitVersion":"v0.15.0-rc.0-51-g8e20b4c71-dirty","gitCommit":"8e20b4c71caa998bd11d1d27a52d4e8d0982a341","buildDate":"2025-11-18T18:14:49Z"}"#;
        let version = extract_version_from_log(log_line);
        assert_eq!(
            version,
            Some("v0.15.0-rc.0-51-g8e20b4c71-dirty".to_string())
        );
    }
}
