//! Kueue CR configuration builder

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Kueue management state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ManagementState {
    Managed,
    Unmanaged,
}

impl Default for ManagementState {
    fn default() -> Self {
        Self::Managed
    }
}

/// Kueue framework integrations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Framework {
    BatchJob,
    Pod,
    Deployment,
    StatefulSet,
    JobSet,
    LeaderWorkerSet,
}

impl Framework {
    pub fn as_str(&self) -> &str {
        match self {
            Framework::BatchJob => "BatchJob",
            Framework::Pod => "Pod",
            Framework::Deployment => "Deployment",
            Framework::StatefulSet => "StatefulSet",
            Framework::JobSet => "JobSet",
            Framework::LeaderWorkerSet => "LeaderWorkerSet",
        }
    }
}

/// Kueue configuration builder
#[derive(Debug, Clone, Default)]
pub struct KueueConfigBuilder {
    name: Option<String>,
    namespace: Option<String>,
    management_state: Option<ManagementState>,
    frameworks: Vec<Framework>,
}

impl KueueConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn management_state(mut self, state: ManagementState) -> Self {
        self.management_state = Some(state);
        self
    }

    pub fn framework(mut self, framework: Framework) -> Self {
        self.frameworks.push(framework);
        self
    }

    pub fn frameworks(mut self, frameworks: Vec<Framework>) -> Self {
        self.frameworks = frameworks;
        self
    }

    pub fn build(self) -> Result<KueueConfig> {
        Ok(KueueConfig {
            name: self.name.unwrap_or_else(|| "cluster".to_string()),
            namespace: self
                .namespace
                .unwrap_or_else(|| "openshift-kueue-operator".to_string()),
            management_state: self.management_state.unwrap_or_default(),
            frameworks: if self.frameworks.is_empty() {
                Self::default_frameworks()
            } else {
                self.frameworks
            },
        })
    }

    fn default_frameworks() -> Vec<Framework> {
        vec![
            Framework::BatchJob,
            Framework::Pod,
            Framework::Deployment,
            Framework::StatefulSet,
            Framework::JobSet,
            Framework::LeaderWorkerSet,
        ]
    }
}

/// Kueue configuration
#[derive(Debug, Clone)]
pub struct KueueConfig {
    pub name: String,
    pub namespace: String,
    pub management_state: ManagementState,
    pub frameworks: Vec<Framework>,
}

impl KueueConfig {
    pub fn builder() -> KueueConfigBuilder {
        KueueConfigBuilder::new()
    }

    /// Generate YAML manifest for Kueue CR
    pub fn to_yaml(&self) -> String {
        let frameworks_yaml = self
            .frameworks
            .iter()
            .map(|f| format!("      - {}", f.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        let management_state = match self.management_state {
            ManagementState::Managed => "Managed",
            ManagementState::Unmanaged => "Unmanaged",
        };

        format!(
            r#"apiVersion: kueue.openshift.io/v1
kind: Kueue
metadata:
  labels:
    app.kubernetes.io/name: kueue-operator
    app.kubernetes.io/managed-by: kustomize
  name: {}
  namespace: {}
spec:
  managementState: {}
  config:
    integrations:
      frameworks:
{}
"#,
            self.name, self.namespace, management_state, frameworks_yaml
        )
    }
}

impl Default for KueueConfig {
    fn default() -> Self {
        KueueConfigBuilder::new().build().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KueueConfig::default();
        assert_eq!(config.name, "cluster");
        assert_eq!(config.namespace, "openshift-kueue-operator");
        assert_eq!(config.management_state, ManagementState::Managed);
        assert_eq!(config.frameworks.len(), 6);
    }

    #[test]
    fn test_builder() {
        let config = KueueConfig::builder()
            .name("test-cluster")
            .namespace("test-namespace")
            .framework(Framework::BatchJob)
            .framework(Framework::Pod)
            .build()
            .unwrap();

        assert_eq!(config.name, "test-cluster");
        assert_eq!(config.namespace, "test-namespace");
        assert_eq!(config.frameworks.len(), 2);
    }

    #[test]
    fn test_yaml_generation() {
        let config = KueueConfig::builder()
            .name("cluster")
            .namespace("openshift-kueue-operator")
            .frameworks(vec![Framework::BatchJob, Framework::Pod])
            .build()
            .unwrap();

        let yaml = config.to_yaml();
        assert!(yaml.contains("name: cluster"));
        assert!(yaml.contains("namespace: openshift-kueue-operator"));
        assert!(yaml.contains("- BatchJob"));
        assert!(yaml.contains("- Pod"));
        assert!(yaml.contains("managementState: Managed"));
    }
}
