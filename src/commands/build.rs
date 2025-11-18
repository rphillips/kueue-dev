//! Build and push container images

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;

use crate::config::images::ImageConfig;
use crate::config::settings::Settings;
use crate::utils::ContainerRuntime;

/// Valid component names that can be built
const VALID_COMPONENTS: &[&str] = &["operator", "operand", "must-gather"];

/// Build and push container images
pub fn build_and_push(components: Vec<String>, images_file: Option<String>, parallel: bool) -> Result<()> {
    crate::log_info!("Building and pushing container images...");

    // Default to all components if none specified
    let components = if components.is_empty() {
        crate::log_info!("No components specified, building all components");
        VALID_COMPONENTS.iter().map(|s| s.to_string()).collect()
    } else {
        components
    };

    // Validate components
    for component in &components {
        if !VALID_COMPONENTS.contains(&component.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid component '{}'. Valid components are: {}",
                component,
                VALID_COMPONENTS.join(", ")
            ));
        }
    }

    // Determine images file path from config or command line
    let images_file_path = if let Some(path) = images_file {
        path
    } else {
        let settings = Settings::load();
        settings.defaults.images_file.clone()
    };

    crate::log_info!("Using images file: {}", images_file_path);

    // Load image configuration
    let images_path = PathBuf::from(&images_file_path);
    let image_config = ImageConfig::load(&images_path)
        .with_context(|| format!("Failed to load image configuration from {}", images_file_path))?;

    // Detect container runtime
    let runtime = ContainerRuntime::detect()?;
    crate::log_info!("Using container runtime: {}", runtime);

    // Get project root (should be kueue-operator directory)
    let project_root = get_project_root()?;

    if parallel {
        crate::log_info!("Building components in parallel...");
        build_parallel(&project_root, &components, &image_config, &runtime)?;
    } else {
        // Build and push each component sequentially
        for component in &components {
            build_and_push_component(&project_root, component, &image_config, &runtime)?;
        }
    }

    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("All images built and pushed successfully!");
    crate::log_info!("==========================================");
    crate::log_info!("");

    Ok(())
}

/// Build and push components in parallel
fn build_parallel(
    project_root: &Path,
    components: &[String],
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    use std::sync::{Arc, Mutex};

    // Set initial progress (0% complete)
    send_progress_update(0, components.len());

    // Create multi-progress for coordinating multiple progress bars
    let multi_progress = Arc::new(MultiProgress::new());

    // Shared error collection
    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Shared completion counter
    let completed: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let total = components.len();

    std::thread::scope(|s| {
        let mut handles = vec![];

        for component in components {
            let component = component.clone();
            let errors = Arc::clone(&errors);
            let completed = Arc::clone(&completed);
            let mp = Arc::clone(&multi_progress);

            let handle = s.spawn(move || {
                use owo_colors::OwoColorize;

                // Create progress indicator for this component
                let pb = mp.add(ProgressBar::new(4));
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                        .template("{spinner:.cyan} {wide_msg}")
                        .unwrap()
                );

                // Set initial message with component name
                pb.set_message(format!("{} {}", component.bright_blue().bold(), "Starting...".dimmed()));

                match build_and_push_component_with_progress(
                    project_root,
                    &component,
                    image_config,
                    runtime,
                    &pb,
                ) {
                    Ok(_) => {
                        pb.finish_with_message(format!("{} {} {}",
                            "✓".bright_green().bold(),
                            component.bright_blue().bold(),
                            "Complete".bright_green()
                        ));

                        // Update completion counter and progress
                        let mut count = completed.lock().unwrap();
                        *count += 1;
                        send_progress_update(*count, total);
                    }
                    Err(e) => {
                        pb.finish_with_message(format!("{} {} {}",
                            "✗".bright_red().bold(),
                            component.bright_blue().bold(),
                            "Failed".bright_red()
                        ));
                        let mut errs = errors.lock().unwrap();
                        errs.push(format!("Failed to build {}: {}", component, e));

                        // Update completion counter and progress
                        let mut count = completed.lock().unwrap();
                        *count += 1;
                        send_progress_update(*count, total);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    });

    // Check if any errors occurred
    let errs = errors.lock().unwrap();
    if !errs.is_empty() {
        clear_progress(false);  // Set error state
        return Err(anyhow::anyhow!(
            "Build failures:\n{}",
            errs.join("\n")
        ));
    }

    clear_progress(true);  // Clear progress indicator

    Ok(())
}

/// Build and push a single component with progress tracking
fn build_and_push_component_with_progress(
    project_root: &Path,
    component: &str,
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
    pb: &indicatif::ProgressBar,
) -> Result<()> {
    use owo_colors::OwoColorize;

    // Step 1: Get image configuration
    pb.set_position(0);
    pb.set_message(format!("{} {}",
        component.bright_blue().bold(),
        "[1/4] Loading config...".dimmed()
    ));

    let image_tag = match component {
        "operator" => image_config.operator()?,
        "operand" => image_config.operand()?,
        "must-gather" => image_config.must_gather()?,
        _ => unreachable!("Component validation should have caught this"),
    };

    // Step 2: Get Dockerfile paths
    pb.inc(1);
    pb.set_message(format!("{} {}",
        component.bright_blue().bold(),
        "[2/4] Locating Dockerfile...".dimmed()
    ));
    let (dockerfile, context) = get_dockerfile_and_context(project_root, component)?;

    // Step 3: Build the image
    pb.inc(1);
    pb.set_message(format!("{} {}",
        component.bright_blue().bold(),
        "[3/4] Building image...".yellow()
    ));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    build_image(runtime, &dockerfile, &context, image_tag)?;

    // Step 4: Push the image
    pb.inc(1);
    pb.set_message(format!("{} {}",
        component.bright_blue().bold(),
        "[4/4] Pushing image...".yellow()
    ));
    push_image(runtime, image_tag)?;

    pb.inc(1);
    Ok(())
}

/// Build and push a single component (sequential mode)
fn build_and_push_component(
    project_root: &Path,
    component: &str,
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
) -> Result<()> {
    crate::log_info!("");
    crate::log_info!("==========================================");
    crate::log_info!("Building component: {}", component);
    crate::log_info!("==========================================");

    // Get the image tag from configuration
    let image_tag = match component {
        "operator" => image_config.operator()?,
        "operand" => image_config.operand()?,
        "must-gather" => image_config.must_gather()?,
        _ => unreachable!("Component validation should have caught this"),
    };

    crate::log_info!("Image tag: {}", image_tag);

    // Get the Dockerfile path and build context
    let (dockerfile, context) = get_dockerfile_and_context(project_root, component)?;

    crate::log_info!("Dockerfile: {}", dockerfile.display());
    crate::log_info!("Build context: {}", context.display());

    // Build the image
    build_image(runtime, &dockerfile, &context, image_tag)?;

    // Push the image
    push_image(runtime, image_tag)?;

    crate::log_info!("Successfully built and pushed: {}", image_tag);

    Ok(())
}

/// Get the Dockerfile path and build context for a component
fn get_dockerfile_and_context(
    project_root: &Path,
    component: &str,
) -> Result<(PathBuf, PathBuf)> {
    match component {
        "operator" => {
            // Operator uses Dockerfile in project root
            Ok((project_root.join("Dockerfile"), project_root.to_path_buf()))
        }
        "operand" => {
            // Operand (kueue) uses Dockerfile.kueue in project root
            Ok((
                project_root.join("Dockerfile.kueue"),
                project_root.to_path_buf(),
            ))
        }
        "must-gather" => {
            // Must-gather has its own directory
            Ok((
                project_root.join("must-gather/Dockerfile"),
                project_root.to_path_buf(),
            ))
        }
        _ => unreachable!("Component validation should have caught this"),
    }
}

/// Build a container image
fn build_image(
    runtime: &ContainerRuntime,
    dockerfile: &Path,
    context: &Path,
    tag: &str,
) -> Result<()> {
    use std::process::Stdio;
    use std::io::BufReader;

    let runtime_cmd = runtime.command();

    let mut cmd = Command::new(runtime_cmd);
    cmd.args(["build", "-f"])
        .arg(dockerfile)
        .args(["-t", tag])
        .arg(context)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to execute {} build command", runtime_cmd))?;

    // Stream output with rolling 5-line buffer
    if let Some(stdout) = child.stdout.take() {
        stream_output_with_rolling_buffer(BufReader::new(stdout), 5);
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to wait for {} build command", runtime_cmd))?;

    if !output.status.success() {
        // On error, always show stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Image build failed for {}:\n{}",
            tag,
            stderr
        ));
    }

    Ok(())
}

/// Push a container image
fn push_image(runtime: &ContainerRuntime, tag: &str) -> Result<()> {
    use std::process::Stdio;
    use std::io::BufReader;

    let runtime_cmd = runtime.command();

    let mut cmd = Command::new(runtime_cmd);
    cmd.args(["push", tag])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to execute {} push command", runtime_cmd))?;

    // Stream output with rolling 5-line buffer
    if let Some(stdout) = child.stdout.take() {
        stream_output_with_rolling_buffer(BufReader::new(stdout), 5);
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to wait for {} push command", runtime_cmd))?;

    if !output.status.success() {
        // On error, always show stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Image push failed for {}:\n{}",
            tag,
            stderr
        ));
    }

    Ok(())
}

/// Stream output with a rolling buffer of N lines
/// This continuously displays the last N lines of output, updating as new lines arrive
fn stream_output_with_rolling_buffer<R: std::io::BufRead>(reader: R, buffer_size: usize) {
    use std::collections::VecDeque;

    let mut buffer: VecDeque<String> = VecDeque::with_capacity(buffer_size);
    let mut lines_displayed = 0;

    for line in reader.lines() {
        if let Ok(line) = line {
            // Add new line to buffer
            if buffer.len() >= buffer_size {
                buffer.pop_front();
            }
            buffer.push_back(line);

            // Clear previous output
            if lines_displayed > 0 {
                // Move cursor up and clear lines
                for _ in 0..lines_displayed {
                    print!("\x1b[1A\x1b[2K"); // Move up one line and clear it
                }
            }

            // Display current buffer
            lines_displayed = buffer.len();
            for line in &buffer {
                println!("{}", line);
            }

            // Flush to ensure immediate display
            let _ = std::io::stdout().flush();
        }
    }

    // Final newline after streaming is complete
    if lines_displayed > 0 {
        println!();
    }
}

/// Get project root directory
fn get_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;

    // Check if we're in kueue-dev directory
    if current_dir.file_name().and_then(|n| n.to_str()) == Some("kueue-dev") {
        // Go up one level to kueue-operator root
        if let Some(parent) = current_dir.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    // Otherwise use current directory
    Ok(current_dir)
}

/// Send OSC 9;4 progress update
/// Format: ESC ] 9 ; 4 ; <state> ; <progress> BEL
/// state: 1 = percentage (0-100)
fn send_progress_update(completed: usize, total: usize) {
    let percentage = if total > 0 {
        (completed as f64 / total as f64 * 100.0) as u32
    } else {
        0
    };

    // Send OSC 9;4 progress report for WezTerm and other supporting terminals
    let _ = std::io::stderr().write_all(format!("\x1b]9;4;1;{}\x07", percentage).as_bytes());
    let _ = std::io::stderr().flush();
}

/// Clear progress indicator using OSC 9;4
/// state: 0 = no progress, 2 = error state
fn clear_progress(success: bool) {
    let state = if success { 0 } else { 2 };
    let _ = std::io::stderr().write_all(format!("\x1b]9;4;{};0\x07", state).as_bytes());
    let _ = std::io::stderr().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_components() {
        assert!(VALID_COMPONENTS.contains(&"operator"));
        assert!(VALID_COMPONENTS.contains(&"operand"));
        assert!(VALID_COMPONENTS.contains(&"must-gather"));
    }

    #[test]
    fn test_build_module() {
        // Basic compile test
        assert!(true);
    }
}
