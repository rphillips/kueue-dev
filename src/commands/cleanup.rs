//! Cleanup command implementation for e2e test resources

use crate::k8s::kubectl;
use anyhow::Result;
use std::path::Path;

/// Clean up e2e test resources
pub fn cleanup(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Cleaning up e2e test resources...");

    // Delete test PriorityClasses (not system ones)
    cleanup_priority_classes(kubeconfig)?;

    // Delete test WorkloadPriorityClasses
    cleanup_resource("workloadpriorityclass", None, kubeconfig)?;

    // Delete all ClusterQueues
    cleanup_resource("clusterqueue", None, kubeconfig)?;

    // Delete all ResourceFlavors
    cleanup_resource("resourceflavor", None, kubeconfig)?;

    // Delete all Cohorts
    cleanup_resource("cohort", None, kubeconfig)?;

    // Delete all AdmissionChecks
    cleanup_resource("admissioncheck", None, kubeconfig)?;

    // Delete workloads in test namespaces
    cleanup_test_workloads(kubeconfig)?;

    // Delete test namespaces
    cleanup_test_namespaces(kubeconfig)?;

    crate::log_info!("Cleanup complete!");
    Ok(())
}

/// Remove finalizers and delete a resource
fn cleanup_resource(
    resource_type: &str,
    namespace: Option<&str>,
    kubeconfig: Option<&Path>,
) -> Result<()> {
    crate::log_info!("Removing finalizers from {}...", resource_type);

    // Get all resources of this type
    let mut args = vec!["get", resource_type, "-o", "name"];
    if let Some(ns) = namespace {
        args.push("-n");
        args.push(ns);
    } else {
        args.push("--all-namespaces");
    }

    let output = kubectl::run_kubectl_output(&args, kubeconfig);

    if let Ok(resources_str) = output {
        let resources: Vec<&str> = resources_str.lines().collect();

        if !resources.is_empty() {
            // Remove finalizers
            for resource in &resources {
                let resource = resource.trim();
                if resource.is_empty() {
                    continue;
                }

                let mut patch_args = vec![
                    "patch",
                    resource,
                    "--type=merge",
                    "-p",
                    r#"{"metadata":{"finalizers":[]}}"#,
                ];
                if let Some(ns) = namespace {
                    patch_args.insert(1, "-n");
                    patch_args.insert(2, ns);
                }

                kubectl::run_kubectl(&patch_args, kubeconfig).ok(); // Ignore errors
            }

            crate::log_info!("Deleting {}...", resource_type);

            // Delete resources
            let mut delete_args = vec!["delete", resource_type, "--all"];
            if let Some(ns) = namespace {
                delete_args.push("-n");
                delete_args.push(ns);
            } else {
                delete_args.push("--all-namespaces");
            }

            kubectl::run_kubectl(&delete_args, kubeconfig).ok(); // Ignore errors
        }
    }

    Ok(())
}

/// Cleanup PriorityClasses (excluding system ones)
fn cleanup_priority_classes(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Removing finalizers from non-system PriorityClasses...");

    let output = kubectl::run_kubectl_output(&["get", "priorityclasses", "-o", "name"], kubeconfig);

    if let Ok(pcs_str) = output {
        let priority_classes: Vec<&str> = pcs_str
            .lines()
            .filter(|line| !line.contains("system-"))
            .collect();

        if !priority_classes.is_empty() {
            // Remove finalizers
            for pc in &priority_classes {
                let pc = pc.trim();
                if pc.is_empty() {
                    continue;
                }

                kubectl::run_kubectl(
                    &[
                        "patch",
                        pc,
                        "--type=merge",
                        "-p",
                        r#"{"metadata":{"finalizers":[]}}"#,
                    ],
                    kubeconfig,
                )
                .ok(); // Ignore errors
            }

            // Delete PriorityClasses
            for pc in &priority_classes {
                let pc = pc.trim();
                if !pc.is_empty() {
                    kubectl::run_kubectl(&["delete", pc], kubeconfig).ok(); // Ignore errors
                }
            }
        }
    }

    Ok(())
}

/// Cleanup workloads in test namespaces
fn cleanup_test_workloads(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Removing finalizers from Workloads in test namespaces...");

    let output = kubectl::run_kubectl_output(&["get", "ns", "-o", "name"], kubeconfig);

    if let Ok(namespaces_str) = output {
        let test_namespaces: Vec<&str> = namespaces_str
            .lines()
            .filter(|line| {
                line.contains("e2e-")
                    || line.contains("sts-e2e-")
                    || line.contains("deployment-e2e-")
                    || line.contains("lws-e2e-")
                    || line.contains("pod-e2e-")
                    || line.contains("jobset-e2e-")
            })
            .collect();

        for ns in test_namespaces {
            let namespace = ns.trim().strip_prefix("namespace/").unwrap_or(ns.trim());
            if namespace.is_empty() {
                continue;
            }

            crate::log_info!("Processing workloads in {}...", namespace);

            let workloads_output = kubectl::run_kubectl_output(
                &["get", "workloads", "-n", namespace, "-o", "name"],
                kubeconfig,
            );

            if let Ok(workloads_str) = workloads_output {
                for workload in workloads_str.lines() {
                    let workload = workload.trim();
                    if workload.is_empty() {
                        continue;
                    }

                    kubectl::run_kubectl(
                        &[
                            "patch",
                            workload,
                            "-n",
                            namespace,
                            "--type=merge",
                            "-p",
                            r#"{"metadata":{"finalizers":[]}}"#,
                        ],
                        kubeconfig,
                    )
                    .ok(); // Ignore errors
                }

                kubectl::run_kubectl(
                    &["delete", "workloads", "-n", namespace, "--all"],
                    kubeconfig,
                )
                .ok();
            }
        }
    }

    Ok(())
}

/// Cleanup test namespaces
fn cleanup_test_namespaces(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Removing finalizers from test namespaces...");

    let output = kubectl::run_kubectl_output(&["get", "ns", "-o", "name"], kubeconfig);

    if let Ok(namespaces_str) = output {
        let test_namespaces: Vec<&str> = namespaces_str
            .lines()
            .filter(|line| {
                line.contains("e2e-")
                    || line.contains("sts-e2e-")
                    || line.contains("deployment-e2e-")
                    || line.contains("lws-e2e-")
                    || line.contains("pod-e2e-")
                    || line.contains("jobset-e2e-")
            })
            .collect();

        if !test_namespaces.is_empty() {
            // Remove finalizers
            for ns in &test_namespaces {
                let ns = ns.trim();
                if ns.is_empty() {
                    continue;
                }

                kubectl::run_kubectl(
                    &[
                        "patch",
                        ns,
                        "--type=merge",
                        "-p",
                        r#"{"metadata":{"finalizers":[]}}"#,
                    ],
                    kubeconfig,
                )
                .ok(); // Ignore errors
            }

            // Delete namespaces
            for ns in &test_namespaces {
                let ns = ns.trim();
                if !ns.is_empty() {
                    kubectl::run_kubectl(&["delete", ns], kubeconfig).ok(); // Ignore errors
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_module() {
        // Basic compile test
        assert!(true);
    }
}
