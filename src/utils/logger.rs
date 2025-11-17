//! Logging utilities using the tracing framework

use std::fmt::Display;

/// Log an informational message (wrapper around tracing::info)
pub fn log_info<T: Display>(msg: T) {
    tracing::info!("{}", msg);
}

/// Log a warning message (wrapper around tracing::warn)
pub fn log_warn<T: Display>(msg: T) {
    tracing::warn!("{}", msg);
}

/// Log an error message (wrapper around tracing::error)
pub fn log_error<T: Display>(msg: T) {
    tracing::error!("{}", msg);
}

/// Macro for convenient info logging
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_info(format!($($arg)*))
    };
}

/// Macro for convenient warning logging
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_warn(format!($($arg)*))
    };
}

/// Macro for convenient error logging
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_error(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_functions() {
        // These should not panic
        log_info("Test info message");
        log_warn("Test warning message");
        log_error("Test error message");
    }
}
