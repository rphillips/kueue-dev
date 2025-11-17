//! Progress indicators for long-running operations

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create a spinner for indeterminate operations
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("Failed to create spinner template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a progress bar for determinate operations
pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .expect("Failed to create progress bar template")
            .progress_chars("█▓▒░ "),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a multi-progress container for parallel operations
pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Progress wrapper for downloads
pub struct DownloadProgress {
    pb: ProgressBar,
}

impl DownloadProgress {
    pub fn new(message: &str) -> Self {
        Self {
            pb: create_spinner(message),
        }
    }

    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    pub fn finish_with_message(&self, message: &str) {
        self.pb.finish_with_message(message.to_string());
    }

    pub fn finish(&self) {
        self.pb.finish_and_clear();
    }
}

/// Progress wrapper for kubectl wait operations
pub struct WaitProgress {
    pb: ProgressBar,
    resource: String,
}

impl WaitProgress {
    pub fn new(resource: &str, condition: &str) -> Self {
        let message = format!("Waiting for {} to be {}", resource, condition);
        Self {
            pb: create_spinner(&message),
            resource: resource.to_string(),
        }
    }

    pub fn update(&self, status: &str) {
        self.pb
            .set_message(format!("{}: {}", self.resource, status));
    }

    pub fn finish_success(&self) {
        self.pb
            .finish_with_message(format!("✓ {} ready", self.resource));
    }

    pub fn finish_error(&self, error: &str) {
        self.pb
            .finish_with_message(format!("✗ {} failed: {}", self.resource, error));
    }

    pub fn finish(&self) {
        self.pb.finish_and_clear();
    }
}

/// Progress wrapper for image loading
pub struct ImageLoadProgress {
    #[allow(dead_code)]
    multi: MultiProgress,
    bars: Vec<ProgressBar>,
}

impl ImageLoadProgress {
    pub fn new(images: &[String]) -> Self {
        let multi = create_multi_progress();
        let bars: Vec<ProgressBar> = images
            .iter()
            .map(|img| {
                let pb = multi.add(create_spinner(&format!("Loading {}", img)));
                pb
            })
            .collect();

        Self { multi, bars }
    }

    pub fn set_image_status(&self, index: usize, status: &str) {
        if let Some(pb) = self.bars.get(index) {
            pb.set_message(status.to_string());
        }
    }

    pub fn finish_image(&self, index: usize, success: bool) {
        if let Some(pb) = self.bars.get(index) {
            if success {
                pb.finish_with_message(format!("✓ {}", pb.message()));
            } else {
                pb.finish_with_message(format!("✗ {}", pb.message()));
            }
        }
    }

    pub fn finish_all(&self) {
        for pb in &self.bars {
            pb.finish_and_clear();
        }
    }
}

/// Progress wrapper for cluster operations
pub struct ClusterProgress {
    pb: ProgressBar,
}

impl ClusterProgress {
    pub fn new(operation: &str, cluster_name: &str) -> Self {
        let message = format!("{} cluster '{}'", operation, cluster_name);
        Self {
            pb: create_spinner(&message),
        }
    }

    pub fn set_step(&self, step: &str) {
        self.pb.set_message(step.to_string());
    }

    pub fn finish_success(&self, message: &str) {
        self.pb.finish_with_message(format!("✓ {}", message));
    }

    pub fn finish_error(&self, message: &str) {
        self.pb.finish_with_message(format!("✗ {}", message));
    }

    pub fn finish(&self) {
        self.pb.finish_and_clear();
    }
}

/// Helper to run a function with a spinner
pub fn with_spinner<F, T>(message: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let pb = create_spinner(message);
    let result = f();
    pb.finish_and_clear();
    result
}

/// Helper to run a function with a spinner and show result
pub fn with_spinner_result<F, T, E>(message: &str, success_msg: &str, f: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
    E: std::fmt::Display,
{
    let pb = create_spinner(message);
    match f() {
        Ok(result) => {
            pb.finish_with_message(format!("✓ {}", success_msg));
            Ok(result)
        }
        Err(e) => {
            pb.finish_with_message(format!("✗ Failed: {}", e));
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_spinner() {
        let pb = create_spinner("Test operation");
        assert!(pb.message().contains("Test operation"));
        pb.finish_and_clear();
    }

    #[test]
    fn test_create_progress_bar() {
        let pb = create_progress_bar(100, "Test progress");
        assert_eq!(pb.length().unwrap(), 100);
        pb.finish_and_clear();
    }

    #[test]
    fn test_download_progress() {
        let dp = DownloadProgress::new("Downloading");
        dp.set_message("Downloading file");
        dp.finish();
    }

    #[test]
    fn test_with_spinner() {
        let result = with_spinner("Testing", || 42);
        assert_eq!(result, 42);
    }
}
