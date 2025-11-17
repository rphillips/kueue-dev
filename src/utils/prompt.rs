//! User prompt utilities for interactive confirmation

use anyhow::Result;
use dialoguer::Confirm;

/// Ask user for yes/no confirmation
pub fn confirm(prompt: &str) -> Result<bool> {
    let result = Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?;

    Ok(result)
}

/// Ask user for yes/no confirmation with default = yes
pub fn confirm_default_yes(prompt: &str) -> Result<bool> {
    let result = Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact()?;

    Ok(result)
}

/// Wait for user to press Enter
pub fn wait_for_enter(message: &str) -> Result<()> {
    use std::io::{self, BufRead};

    println!("{}", message);
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    lines.next();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_module_exists() {
        // Basic compile test - actual prompts can't be tested in CI
        assert!(true);
    }
}
