//! Enhanced error types with actionable suggestions

use colored::Colorize;
use thiserror::Error;

/// Enhanced error with suggestions and documentation links
#[derive(Error, Debug)]
#[error("{message}")]
pub struct KueueDevError {
    pub message: String,
    pub suggestions: Vec<String>,
    pub docs_link: Option<String>,
}

impl KueueDevError {
    /// Create a new error with suggestions
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            suggestions: Vec::new(),
            docs_link: None,
        }
    }

    /// Add a suggestion to the error
    pub fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Add a documentation link
    pub fn with_docs(mut self, link: impl Into<String>) -> Self {
        self.docs_link = Some(link.into());
        self
    }

    /// Display the error with suggestions
    pub fn display(&self) {
        crate::log_error!("{}", self.message);

        if !self.suggestions.is_empty() {
            println!();
            println!("{}", "Suggestions:".yellow().bold());
            for suggestion in &self.suggestions {
                println!("  {} {}", "→".blue(), suggestion);
            }
        }

        if let Some(docs) = &self.docs_link {
            println!();
            println!("{} {}", "📚 Documentation:".cyan(), docs);
        }
    }

    // Common error patterns

    /// Cluster not found error
    pub fn cluster_not_found(name: &str) -> Self {
        Self::new(format!("Kind cluster '{}' not found", name))
            .suggest(format!(
                "Create cluster with: kueue-dev cluster create --name {}",
                name
            ))
            .suggest("List existing clusters with: kueue-dev cluster list")
    }

    /// Kubeconfig not found error
    pub fn kubeconfig_not_found(path: &str) -> Self {
        Self::new(format!("Kubeconfig not found: {}", path))
            .suggest("Verify the cluster exists")
            .suggest("Check if you have the correct cluster name")
            .suggest("Use --kubeconfig flag to specify a custom kubeconfig")
    }

    /// Tool not found error
    pub fn tool_not_found(tool: &str, install_hint: &str) -> Self {
        Self::new(format!("Required tool '{}' not found", tool))
            .suggest(format!("Install with: {}", install_hint))
            .suggest("Ensure the tool is in your PATH")
    }

    /// Image not found error
    pub fn image_not_found(image: &str) -> Self {
        Self::new(format!("Image not found: {}", image))
            .suggest("Verify the image name is correct")
            .suggest("Check if the image exists in the registry")
            .suggest("Run with --verbose to see more details")
    }

    /// Image config file error
    pub fn image_config_error(path: &str, reason: &str) -> Self {
        Self::new(format!(
            "Failed to load image config from {}: {}",
            path, reason
        ))
        .suggest("Verify the file exists and is readable")
        .suggest("Check that the JSON format is valid")
        .suggest("Example format: {\"operator\": \"quay.io/org/operator:tag\", ...}")
    }

    /// Deployment not ready error
    pub fn deployment_not_ready(name: &str, namespace: &str) -> Self {
        Self::new(format!(
            "Deployment {}/{} failed to become ready",
            namespace, name
        ))
        .suggest(format!(
            "Check pod status: kubectl get pods -n {}",
            namespace
        ))
        .suggest(format!(
            "View logs: kubectl logs -n {} -l app={}",
            namespace, name
        ))
        .suggest("Increase timeout with --timeout flag")
    }

    /// Permission denied error
    pub fn permission_denied(operation: &str) -> Self {
        Self::new(format!("Permission denied: {}", operation))
            .suggest("Verify you have sufficient cluster permissions")
            .suggest("Check if you need cluster-admin role")
            .suggest("For OpenShift: ensure you're logged in as a privileged user")
    }

    /// Test failure error
    pub fn test_failed(reason: &str) -> Self {
        Self::new(format!("Tests failed: {}", reason))
            .suggest("Review test output above for specific failures")
            .suggest("Run with --focus to test specific features")
            .suggest("Check operator logs: kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator")
    }

    /// Namespace conflict error
    pub fn namespace_conflict(namespace: &str) -> Self {
        Self::new(format!(
            "Namespace '{}' already exists with conflicting resources",
            namespace
        ))
        .suggest("Clean up existing resources: kueue-dev cleanup".to_string())
        .suggest(format!(
            "Delete namespace: kubectl delete namespace {}",
            namespace
        ))
        .suggest("Use a different cluster or namespace")
    }

    /// Connection timeout error
    pub fn connection_timeout(resource: &str) -> Self {
        Self::new(format!("Timeout waiting for {}", resource))
            .suggest("Check if the cluster is healthy")
            .suggest("Verify network connectivity")
            .suggest("Increase timeout value")
            .suggest("Check for pending pods: kubectl get pods --all-namespaces")
    }

    /// Version mismatch error
    pub fn version_mismatch(tool: &str, current: &str, required: &str) -> Self {
        Self::new(format!(
            "{} version {} does not meet requirement {}",
            tool, current, required
        ))
        .suggest(format!("Upgrade {} to version {}", tool, required))
        .suggest("Check installation instructions for your platform")
    }

    /// Missing prerequisite error
    pub fn missing_prerequisite(prereq: &str, reason: &str) -> Self {
        Self::new(format!("Missing prerequisite: {} ({})", prereq, reason))
            .suggest("Run 'kueue-dev check' to see all prerequisites")
            .suggest(format!("Install {}", prereq))
    }

    /// OpenShift not logged in error
    pub fn openshift_not_logged_in() -> Self {
        Self::new("Not logged into an OpenShift cluster")
            .suggest("Log in with: oc login <cluster-url>")
            .suggest("Verify credentials and cluster accessibility")
    }

    /// OLM not installed error
    pub fn olm_not_installed() -> Self {
        Self::new("OLM (Operator Lifecycle Manager) is not installed on the cluster")
            .suggest("OLM will be automatically installed during deployment")
            .suggest("Or manually install with: operator-sdk olm install")
    }
}

/// Helper to display error and exit
pub fn display_error_and_exit(error: KueueDevError) -> ! {
    error.display();
    std::process::exit(1);
}

/// Convert anyhow error to KueueDevError when possible
pub fn enhance_error(err: anyhow::Error) -> KueueDevError {
    let err_str = err.to_string();

    // Pattern match common kubectl errors
    if err_str.contains("not found") && err_str.contains("cluster") {
        let cluster_name = extract_cluster_name(&err_str).unwrap_or("unknown");
        return KueueDevError::cluster_not_found(cluster_name);
    }

    if err_str.contains("connection refused") || err_str.contains("timeout") {
        return KueueDevError::connection_timeout("cluster");
    }

    if err_str.contains("unauthorized") || err_str.contains("forbidden") {
        return KueueDevError::permission_denied("cluster operation");
    }

    if err_str.contains("not logged") {
        return KueueDevError::openshift_not_logged_in();
    }

    // Default error with generic suggestion
    KueueDevError::new(err_str)
        .suggest("Run with --verbose for more details")
        .suggest("Check logs for additional context")
}

/// Extract cluster name from error message
fn extract_cluster_name(msg: &str) -> Option<&str> {
    // Try to extract cluster name from common error patterns
    if let Some(start) = msg.find("cluster '")
        && let Some(end) = msg[start + 9..].find('\'')
    {
        return Some(&msg[start + 9..start + 9 + end]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_not_found_error() {
        let err = KueueDevError::cluster_not_found("my-cluster");
        assert!(err.message.contains("my-cluster"));
        assert_eq!(err.suggestions.len(), 2);
    }

    #[test]
    fn test_error_with_docs() {
        let err = KueueDevError::new("test error").with_docs("https://example.com");
        assert!(err.docs_link.is_some());
    }

    #[test]
    fn test_error_suggestions() {
        let err = KueueDevError::new("test")
            .suggest("suggestion 1")
            .suggest("suggestion 2");
        assert_eq!(err.suggestions.len(), 2);
    }
}
