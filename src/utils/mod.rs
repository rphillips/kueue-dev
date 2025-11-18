//! Utility modules for kueue-dev

pub mod container;
pub mod errors;
pub mod logger;
pub mod preflight;
pub mod prereqs;
pub mod progress;
pub mod prompt;

// Re-export commonly used items
pub use container::ContainerRuntime;
pub use errors::{display_error_and_exit, enhance_error, KueueDevError};
pub use logger::{log_error, log_info, log_warn};
pub use preflight::{run_preflight_with_confirm, PreflightChecker};
pub use prereqs::{CommonPrereqs, Prerequisite};
pub use progress::{create_progress_bar, create_spinner, with_spinner, with_spinner_result};
pub use prompt::{confirm, confirm_default_yes, wait_for_enter};
