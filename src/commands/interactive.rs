//! Interactive menu for debugging and cluster management

use anyhow::Result;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::k8s::kubectl;

/// Show interactive menu for cluster operations
pub fn show_menu(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Interactive Menu");
    crate::log_info!("==========================================");
    crate::log_info!("");

    loop {
        println!();
        println!("Available actions:");
        println!("  1) Port-forward to Prometheus UI (http://localhost:9090)");
        println!("  2) View Prometheus Operator logs");
        println!("  3) View Prometheus instance logs");
        println!("  4) View Kueue Operator logs");
        println!("  5) Show cluster information");
        println!("  6) kubectl shell (interactive)");
        println!("  7) Exit");
        println!();
        print!("Select an action [1-7]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => port_forward_prometheus(kubeconfig)?,
            "2" => view_prometheus_operator_logs(kubeconfig)?,
            "3" => view_prometheus_logs(kubeconfig)?,
            "4" => view_kueue_logs(kubeconfig)?,
            "5" => show_cluster_info(kubeconfig)?,
            "6" => kubectl_shell(kubeconfig)?,
            "7" => {
                crate::log_info!("Exiting...");
                break;
            }
            _ => {
                crate::log_error!("Invalid selection. Please choose 1-7.");
            }
        }
    }

    Ok(())
}

/// Port-forward to Prometheus UI
fn port_forward_prometheus(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Starting port-forward to Prometheus UI...");
    crate::log_info!("Access Prometheus at: http://localhost:9090");
    crate::log_info!("Press Ctrl+C to stop port-forwarding and return to menu");

    let mut cmd = Command::new("kubectl");
    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args([
        "port-forward",
        "-n",
        "openshift-monitoring",
        "svc/prometheus",
        "9090:9090",
    ]);

    let _ = cmd.status(); // Ignore error from Ctrl+C

    Ok(())
}

/// View Prometheus Operator logs
fn view_prometheus_operator_logs(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Showing Prometheus Operator logs...");
    crate::log_info!("Press Ctrl+C to stop and return to menu");

    let mut cmd = Command::new("kubectl");
    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args([
        "logs",
        "-n",
        "default",
        "-l",
        "app.kubernetes.io/name=prometheus-operator",
        "-f",
        "--tail=100",
    ]);

    let _ = cmd.status(); // Ignore error from Ctrl+C

    Ok(())
}

/// View Prometheus instance logs
fn view_prometheus_logs(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Showing Prometheus instance logs...");
    crate::log_info!("Press Ctrl+C to stop and return to menu");

    let mut cmd = Command::new("kubectl");
    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args([
        "logs",
        "-n",
        "openshift-monitoring",
        "-l",
        "app.kubernetes.io/name=prometheus",
        "-f",
        "--tail=100",
    ]);

    let _ = cmd.status(); // Ignore error from Ctrl+C

    Ok(())
}

/// View Kueue Operator logs
fn view_kueue_logs(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Showing Kueue Operator logs...");
    crate::log_info!("Press Ctrl+C to stop and return to menu");

    let mut cmd = Command::new("kubectl");
    if let Some(kc) = kubeconfig {
        cmd.env("KUBECONFIG", kc);
    }

    cmd.args([
        "logs",
        "-n",
        "openshift-kueue-operator",
        "-l",
        "name=openshift-kueue-operator",
        "-f",
        "--tail=100",
    ]);

    let _ = cmd.status(); // Ignore error from Ctrl+C

    Ok(())
}

/// Show cluster information
fn show_cluster_info(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Cluster Information:");
    println!();

    // Show Prometheus Operator deployment
    crate::log_info!("Prometheus Operator Deployment (default namespace):");
    kubectl::run_kubectl(
        &["get", "deployment", "-n", "default", "prometheus-operator"],
        kubeconfig,
    )
    .ok();
    println!();

    // Show Prometheus pods
    crate::log_info!("Prometheus Pods (openshift-monitoring namespace):");
    kubectl::run_kubectl(
        &[
            "get",
            "pods",
            "-n",
            "openshift-monitoring",
            "-l",
            "app.kubernetes.io/name=prometheus",
        ],
        kubeconfig,
    )
    .ok();
    println!();

    // Show Prometheus service
    crate::log_info!("Prometheus Service:");
    kubectl::run_kubectl(
        &["get", "svc", "-n", "openshift-monitoring", "prometheus"],
        kubeconfig,
    )
    .ok();
    println!();

    // Show Kueue Operator deployment
    crate::log_info!("Kueue Operator Deployment:");
    kubectl::run_kubectl(
        &[
            "get",
            "deployment",
            "-n",
            "openshift-kueue-operator",
            "openshift-kueue-operator",
        ],
        kubeconfig,
    )
    .ok();
    println!();

    // Show Kueue Operator pods
    crate::log_info!("Kueue Operator Pods:");
    kubectl::run_kubectl(
        &["get", "pods", "-n", "openshift-kueue-operator"],
        kubeconfig,
    )
    .ok();
    println!();

    Ok(())
}

/// Interactive kubectl shell
fn kubectl_shell(kubeconfig: Option<&Path>) -> Result<()> {
    crate::log_info!("Starting kubectl shell...");
    crate::log_info!("Type 'exit' to return to menu");
    println!();

    loop {
        print!("kubectl> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        if input == "exit" || input.is_empty() {
            break;
        }

        // Split input into args
        let args: Vec<&str> = input.split_whitespace().collect();

        if args.is_empty() {
            continue;
        }

        let mut cmd = Command::new("kubectl");
        if let Some(kc) = kubeconfig {
            cmd.env("KUBECONFIG", kc);
        }

        cmd.args(&args);
        let _ = cmd.status(); // Ignore errors
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_interactive_module() {
        // Basic compile test
    }
}
