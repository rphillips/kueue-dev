//! Image configuration parsing from related_images.json

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedImage {
    pub name: String,
    pub image: String,
}

#[derive(Debug, Clone)]
pub struct ImageConfig {
    images: HashMap<String, String>,
}

impl ImageConfig {
    /// Load image configuration from JSON file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read image config file: {}", path.display()))?;

        let images: Vec<RelatedImage> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse image config: {}", path.display()))?;

        let mut image_map = HashMap::new();
        for img in images {
            image_map.insert(img.name.clone(), img.image);
        }

        Ok(Self { images: image_map })
    }

    /// Get image URL by name
    pub fn get(&self, name: &str) -> Result<&str> {
        self.images
            .get(name)
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("Image '{}' not found in configuration", name))
    }

    /// Get operator image
    pub fn operator(&self) -> Result<&str> {
        self.get("operator")
    }

    /// Get operand image
    pub fn operand(&self) -> Result<&str> {
        self.get("operand")
    }

    /// Get must-gather image
    pub fn must_gather(&self) -> Result<&str> {
        self.get("must-gather")
    }

    /// List all images
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.images
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_image_config() {
        let json = r#"[
            {
                "name": "operator",
                "image": "quay.io/example/operator:latest"
            },
            {
                "name": "operand",
                "image": "quay.io/example/operand:latest"
            }
        ]"#;

        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(json.as_bytes()).unwrap();

        let config = ImageConfig::load(temp.path()).unwrap();
        assert_eq!(
            config.operator().unwrap(),
            "quay.io/example/operator:latest"
        );
        assert_eq!(config.operand().unwrap(), "quay.io/example/operand:latest");
    }

    #[test]
    fn test_missing_image() {
        let json = r#"[{"name": "operator", "image": "quay.io/example/operator:latest"}]"#;

        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(json.as_bytes()).unwrap();

        let config = ImageConfig::load(temp.path()).unwrap();
        assert!(config.get("nonexistent").is_err());
    }
}
