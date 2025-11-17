//! Dry-run mode utilities

use colored::Colorize;
use std::env;

/// Check if dry-run mode is enabled
pub fn is_dry_run() -> bool {
    env::var("KUEUE_DEV_DRY_RUN").is_ok()
}

/// Log a dry-run action
pub fn log_action(action: &str) {
    if is_dry_run() {
        println!("  {} {}", "[DRY RUN]".cyan().bold(), action);
    }
}

/// Log multiple dry-run actions as a numbered list
pub fn log_actions(actions: &[String]) {
    if !is_dry_run() {
        return;
    }

    println!(
        "{}",
        "[DRY RUN] Would perform the following actions:"
            .cyan()
            .bold()
    );
    println!();

    for (i, action) in actions.iter().enumerate() {
        println!("  {}. {}", i + 1, action);
    }

    println!();
    println!("{}", "No changes were made (--dry-run mode)".yellow());
}

/// Execute function only if not in dry-run mode
/// Returns Ok(()) in dry-run mode without executing
pub fn exec_unless_dry_run<F>(action_desc: &str, f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    if is_dry_run() {
        log_action(action_desc);
        Ok(())
    } else {
        f()
    }
}

/// Execute function and return value only if not in dry-run mode
/// Returns default value in dry-run mode
pub fn exec_unless_dry_run_with_default<F, T>(
    action_desc: &str,
    default: T,
    f: F,
) -> anyhow::Result<T>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    if is_dry_run() {
        log_action(action_desc);
        Ok(default)
    } else {
        f()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dry_run_default() {
        // Clear the env var first
        env::remove_var("KUEUE_DEV_DRY_RUN");
        assert!(!is_dry_run());
    }

    #[test]
    fn test_is_dry_run_enabled() {
        env::set_var("KUEUE_DEV_DRY_RUN", "1");
        assert!(is_dry_run());
        env::remove_var("KUEUE_DEV_DRY_RUN");
    }

    #[test]
    fn test_exec_unless_dry_run() {
        env::remove_var("KUEUE_DEV_DRY_RUN");

        let mut executed = false;
        let result = exec_unless_dry_run("test action", || {
            executed = true;
            Ok(())
        });

        assert!(result.is_ok());
        assert!(executed);
    }

    #[test]
    fn test_exec_unless_dry_run_in_dry_run_mode() {
        env::set_var("KUEUE_DEV_DRY_RUN", "1");

        let mut executed = false;
        let result = exec_unless_dry_run("test action", || {
            executed = true;
            Ok(())
        });

        assert!(result.is_ok());
        assert!(!executed); // Should not execute in dry-run mode

        env::remove_var("KUEUE_DEV_DRY_RUN");
    }
}
