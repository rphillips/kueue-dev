//! Prometheus operator installation

use crate::k8s::kubectl;
use anyhow::{Context, Result};
use std::path::Path;

/// Install Prometheus operator and create a Prometheus instance
pub fn install(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing Prometheus Operator {}...", version);

    // Check if prometheus-operator is already installed
    let ns_check = kubectl::run_kubectl_output(
        &["get", "deployment", "prometheus-operator", "-n", "default"],
        kubeconfig,
    );

    if ns_check.is_ok() {
        crate::log_info!("Prometheus Operator already installed, skipping installation");
        return Ok(());
    }

    let bundle_url = format!(
        "https://github.com/prometheus-operator/prometheus-operator/releases/download/{}/bundle.yaml",
        version
    );

    crate::log_info!("Downloading Prometheus Operator manifest...");

    // Download and apply the bundle (includes CRDs and operator)
    let bundle_yaml = reqwest::blocking::get(&bundle_url)
        .context("Failed to download Prometheus Operator bundle")?
        .text()
        .context("Failed to read Prometheus Operator bundle")?;

    // Use server-side apply to avoid annotation size limits
    let mut temp_file = tempfile::NamedTempFile::new()?;
    use std::io::Write;
    temp_file.write_all(bundle_yaml.as_bytes())?;
    temp_file.flush()?;

    kubectl::run_kubectl(
        &[
            "apply",
            "--server-side",
            "-f",
            temp_file.path().to_str().unwrap(),
        ],
        kubeconfig,
    )?;

    crate::log_info!("Waiting for Prometheus Operator deployment to be created...");

    // Wait for deployment to exist
    std::thread::sleep(std::time::Duration::from_secs(5));

    crate::log_info!("Configuring Prometheus Operator with debug logging...");

    // Patch deployment to add debug log level
    kubectl::run_kubectl(
        &[
            "patch",
            "deployment",
            "prometheus-operator",
            "-n",
            "default",
            "--type=json",
            "-p",
            r#"[{"op":"add","path":"/spec/template/spec/containers/0/args/-","value":"--log-level=debug"}]"#,
        ],
        kubeconfig,
    )
    .ok(); // Ignore errors if already patched

    crate::log_info!("Waiting for Prometheus Operator to be ready...");
    kubectl::wait_for_condition(
        "deployment/prometheus-operator",
        "condition=Available",
        Some("default"),
        "300s",
        kubeconfig,
    )?;

    crate::log_info!("Prometheus Operator installed successfully");

    // Now create the Prometheus instance
    create_prometheus_instance(kubeconfig)?;

    Ok(())
}

/// Create Prometheus instance with RBAC
fn create_prometheus_instance(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Creating Prometheus instance...");

    // Service Account
    let service_account_yaml = r#"apiVersion: v1
kind: ServiceAccount
metadata:
  name: prometheus
"#;

    // Cluster Role
    let cluster_role_yaml = r#"apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: prometheus
rules:
- apiGroups: [""]
  resources:
  - nodes
  - nodes/metrics
  - services
  - endpoints
  - pods
  verbs: ["get", "list", "watch"]
- apiGroups: [""]
  resources:
  - configmaps
  verbs: ["get"]
- apiGroups:
  - discovery.k8s.io
  resources:
  - endpointslices
  verbs: ["get", "list", "watch"]
- apiGroups:
  - networking.k8s.io
  resources:
  - ingresses
  verbs: ["get", "list", "watch"]
- nonResourceURLs: ["/metrics"]
  verbs: ["get"]
"#;

    // Cluster Role Binding
    let cluster_role_binding_yaml = r#"apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: prometheus
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: prometheus
subjects:
- kind: ServiceAccount
  name: prometheus
  namespace: default
"#;

    // Prometheus Instance
    let prometheus_instance_yaml = r#"apiVersion: monitoring.coreos.com/v1
kind: Prometheus
metadata:
  name: prometheus
spec:
  scrapeInterval: "5s"
  logLevel: "debug"
  serviceAccountName: prometheus
  serviceMonitorSelector: {}
  serviceMonitorNamespaceSelector: {}
"#;

    // Apply all resources
    kubectl::apply_yaml(service_account_yaml, kubeconfig)
        .context("Failed to create Prometheus ServiceAccount")?;

    kubectl::apply_yaml(cluster_role_yaml, kubeconfig)
        .context("Failed to create Prometheus ClusterRole")?;

    kubectl::apply_yaml(cluster_role_binding_yaml, kubeconfig)
        .context("Failed to create Prometheus ClusterRoleBinding")?;

    kubectl::apply_yaml(prometheus_instance_yaml, kubeconfig)
        .context("Failed to create Prometheus instance")?;

    crate::log_info!("Waiting for Prometheus pods to be ready...");

    // Wait for the statefulset and pods
    kubectl::wait_for_condition(
        "pod",
        "condition=ready",
        Some("default"),
        "300s",
        kubeconfig,
    )
    .ok(); // Ignore errors, might take time

    crate::log_info!("Prometheus instance created successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_prometheus_module() {
        // Basic compile test
    }
}
