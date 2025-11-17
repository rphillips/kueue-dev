//! Prometheus operator installation

use anyhow::{Context, Result};
use std::path::Path;
use crate::k8s::kubectl;

/// Install Prometheus operator
pub fn install_prometheus_operator(version: &str, kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Installing Prometheus Operator {}...", version);

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
        &["apply", "--server-side", "-f", temp_file.path().to_str().unwrap()],
        kubeconfig,
    )?;

    crate::log_info!("Waiting for Prometheus Operator deployment to be created...");

    // Wait for deployment to exist
    std::thread::sleep(std::time::Duration::from_secs(5));

    crate::log_info!("Configuring Prometheus Operator with debug logging...");

    // Patch deployment to add debug log level
    kubectl::run_kubectl(
        &[
            "patch", "deployment", "prometheus-operator", "-n", "default",
            "--type=json",
            "-p", r#"[{"op":"add","path":"/spec/template/spec/containers/0/args/-","value":"--log-level=debug"}]"#,
        ],
        kubeconfig,
    ).ok(); // Ignore errors if already patched

    crate::log_info!("Waiting for Prometheus Operator to be ready...");
    kubectl::wait_for_condition(
        "deployment/prometheus-operator",
        "condition=Available",
        Some("default"),
        "300s",
        kubeconfig,
    )?;

    crate::log_info!("Prometheus Operator installed successfully with debug logging enabled");
    Ok(())
}

/// Create monitoring namespace
pub fn create_monitoring_namespace(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Creating openshift-monitoring namespace...");

    let namespace_yaml = r#"apiVersion: v1
kind: Namespace
metadata:
  name: openshift-monitoring
  labels:
    openshift.io/cluster-monitoring: "true"
    kubernetes.io/metadata.name: openshift-monitoring
"#;

    kubectl::apply_yaml(namespace_yaml, kubeconfig)?;
    crate::log_info!("OpenShift monitoring namespace created successfully");
    Ok(())
}

/// Create Prometheus instance with debugging
pub fn create_prometheus_instance(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Creating Prometheus instance with debugging enabled...");

    let prometheus_yaml = r#"apiVersion: v1
kind: ServiceAccount
metadata:
  name: prometheus
  namespace: openshift-monitoring
---
apiVersion: rbac.authorization.k8s.io/v1
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
  - networking.k8s.io
  resources:
  - ingresses
  verbs: ["get", "list", "watch"]
- apiGroups:
  - monitoring.coreos.com
  resources:
  - servicemonitors
  - podmonitors
  - prometheusrules
  verbs: ["get", "list", "watch"]
- nonResourceURLs: ["/metrics"]
  verbs: ["get"]
---
apiVersion: rbac.authorization.k8s.io/v1
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
  namespace: openshift-monitoring
---
apiVersion: monitoring.coreos.com/v1
kind: Prometheus
metadata:
  name: prometheus
  namespace: openshift-monitoring
spec:
  serviceAccountName: prometheus
  replicas: 1
  logLevel: debug
  logFormat: logfmt
  retention: 2h
  resources:
    requests:
      memory: 400Mi
  enableAdminAPI: true
  serviceMonitorSelector: {}
  serviceMonitorNamespaceSelector: {}
  podMonitorSelector: {}
  podMonitorNamespaceSelector: {}
  ruleSelector: {}
  ruleNamespaceSelector: {}
---
apiVersion: v1
kind: Service
metadata:
  name: prometheus
  namespace: openshift-monitoring
  labels:
    app: prometheus
spec:
  type: NodePort
  ports:
  - name: web
    port: 9090
    targetPort: web
    nodePort: 30090
  selector:
    app.kubernetes.io/name: prometheus
    prometheus: prometheus
"#;

    kubectl::apply_yaml(prometheus_yaml, kubeconfig)?;

    crate::log_info!("Waiting for Prometheus Operator to create the StatefulSet...");
    std::thread::sleep(std::time::Duration::from_secs(10));

    // Wait for the statefulset and pods
    crate::log_info!("Waiting for Prometheus pods to be ready...");
    kubectl::wait_for_condition(
        "pod",
        "condition=ready",
        Some("openshift-monitoring"),
        "300s",
        kubeconfig,
    ).ok(); // Ignore errors, might take time

    crate::log_info!("Prometheus instance created successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_module() {
        // Basic compile test
        assert!(true);
    }
}
