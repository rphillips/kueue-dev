//! Build and push container images

use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::images::ImageConfig;
use crate::config::settings::Settings;
use crate::utils::ContainerRuntime;

/// Valid component names that can be built
const VALID_COMPONENTS: &[&str] = &["operator", "operand", "must-gather", "bundle"];

/// Build and push container images
pub fn build_and_push(
    components: Vec<String>,
    images_file: Option<String>,
    parallel: bool,
) -> Result<()> {
    // Load settings BEFORE changing directories
    // This ensures we read the config from where the user is running the command
    let images_file_path = if let Some(path) = images_file {
        path
    } else {
        let settings = Settings::load();
        settings.defaults.images_file.clone()
    };

    // Ensure we're in the operator source directory
    let source_path = crate::utils::ensure_operator_source_directory()?;

    crate::log_info!("Building and pushing container images...");
    crate::log_info!("Kueue source path: {}", source_path.display());

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

    crate::log_info!("Using images file: {}", images_file_path);

    // Load image configuration
    let images_path = PathBuf::from(&images_file_path);
    let image_config = ImageConfig::load(&images_path).with_context(|| {
        format!(
            "Failed to load image configuration from {}",
            images_file_path
        )
    })?;

    // Detect container runtime
    let runtime = ContainerRuntime::detect()?;
    crate::log_info!("Using container runtime: {}", runtime);

    if parallel {
        crate::log_info!("Building components in parallel...");
        build_parallel(&components, &image_config, &runtime, &images_file_path)?;
    } else {
        // Build and push each component sequentially
        for component in &components {
            build_and_push_component(component, &image_config, &runtime, &images_file_path)?;
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
    components: &[String],
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
    images_file_path: &str,
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
            let images_file_path = images_file_path.to_string();

            let handle = s.spawn(move || {
                use owo_colors::OwoColorize;

                // Create progress indicator for this component
                let pb = mp.add(ProgressBar::new(4));
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                        .template("{spinner:.cyan} {wide_msg}")
                        .unwrap(),
                );

                // Set initial message with component name
                pb.set_message(format!(
                    "{} {}",
                    component.bright_blue().bold(),
                    "Starting...".dimmed()
                ));

                match build_and_push_component_with_progress(
                    &component,
                    image_config,
                    runtime,
                    &pb,
                    &images_file_path,
                ) {
                    Ok(_) => {
                        pb.finish_and_clear();
                        mp.println(format!(
                            "{} {} {}",
                            "✅".bright_green(),
                            component.bright_blue().bold(),
                            "Complete".bright_green()
                        )).unwrap();

                        // Update completion counter and progress
                        let mut count = completed.lock().unwrap();
                        *count += 1;
                        send_progress_update(*count, total);
                    }
                    Err(e) => {
                        pb.finish_and_clear();
                        mp.println(format!(
                            "{} {} {}",
                            "✗".bright_red().bold(),
                            component.bright_blue().bold(),
                            "Failed".bright_red()
                        )).unwrap();
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
        clear_progress(false); // Set error state
        return Err(anyhow::anyhow!("Build failures:\n{}", errs.join("\n")));
    }

    clear_progress(true); // Clear progress indicator

    Ok(())
}

/// Build and push a single component with progress tracking
fn build_and_push_component_with_progress(
    component: &str,
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
    pb: &indicatif::ProgressBar,
    images_file_path: &str,
) -> Result<()> {
    use owo_colors::OwoColorize;

    // Step 1: Get image configuration
    pb.set_position(0);
    pb.set_message(format!(
        "{} {}",
        component.bright_blue().bold(),
        "[1/4] Loading config...".dimmed()
    ));

    let image_tag = match component {
        "operator" => image_config.operator()?,
        "operand" => image_config.operand()?,
        "must-gather" => image_config.must_gather()?,
        "bundle" => image_config.bundle()?,
        _ => unreachable!("Component validation should have caught this"),
    };

    // Step 2: Get Dockerfile paths
    pb.inc(1);
    pb.set_message(format!(
        "{} {}",
        component.bright_blue().bold(),
        "[2/4] Locating Dockerfile...".dimmed()
    ));
    let (dockerfile, context) = get_dockerfile_and_context(component)?;

    // Step 3: Build the image
    pb.inc(1);
    pb.set_message(format!(
        "{} {}",
        component.bright_blue().bold(),
        "[3/4] Building image...".yellow()
    ));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Get build args for this component
    let build_args = get_build_args(component, image_config, images_file_path)?;
    build_image(runtime, &dockerfile, &context, image_tag, &build_args)?;

    // Step 4: Push the image
    pb.inc(1);
    pb.set_message(format!(
        "{} {}",
        component.bright_blue().bold(),
        "[4/4] Pushing image...".yellow()
    ));
    push_image(runtime, image_tag)?;

    pb.inc(1);
    Ok(())
}

/// Build and push a single component (sequential mode)
fn build_and_push_component(
    component: &str,
    image_config: &ImageConfig,
    runtime: &ContainerRuntime,
    images_file_path: &str,
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
        "bundle" => image_config.bundle()?,
        _ => unreachable!("Component validation should have caught this"),
    };

    crate::log_info!("Image tag: {}", image_tag);

    // Get the Dockerfile path and build context
    let (dockerfile, context) = get_dockerfile_and_context(component)?;

    crate::log_info!("Dockerfile: {}", dockerfile.display());
    crate::log_info!("Build context: {}", context.display());

    // Get build args for this component
    let build_args = get_build_args(component, image_config, images_file_path)?;
    if !build_args.is_empty() {
        crate::log_info!("Build args: {:?}", build_args);
    }

    // Build the image
    build_image(runtime, &dockerfile, &context, image_tag, &build_args)?;

    // Push the image
    push_image(runtime, image_tag)?;

    crate::log_info!("Successfully built and pushed: {}", image_tag);

    Ok(())
}

/// Get build arguments for a component
fn get_build_args(
    component: &str,
    _image_config: &ImageConfig,
    images_file_path: &str,
) -> Result<Vec<(String, String)>> {
    match component {
        "bundle" => {
            // Bundle needs RELATED_IMAGE_FILE build arg
            // The Dockerfile expects just the filename (e.g., "related_images.json"),
            // not a full path, since it will COPY from the build context
            let images_file_path = PathBuf::from(images_file_path);

            // Get just the filename
            let images_file = images_file_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid images file path"))?
                .to_string();

            Ok(vec![("RELATED_IMAGE_FILE".to_string(), images_file)])
        }
        _ => {
            // No build args needed for other components
            Ok(vec![])
        }
    }
}

/// Get the Dockerfile path and build context for a component
fn get_dockerfile_and_context(component: &str) -> Result<(PathBuf, PathBuf)> {
    // We're already in the operator source directory after ensure_operator_source_directory()
    let source_path = std::env::current_dir().context("Failed to get current directory")?;

    match component {
        "operator" => {
            // Operator uses Dockerfile in project root
            Ok((source_path.join("Dockerfile"), source_path.clone()))
        }
        "operand" => {
            // Operand (kueue) uses Dockerfile.kueue in project root
            Ok((source_path.join("Dockerfile.kueue"), source_path.clone()))
        }
        "must-gather" => {
            // Must-gather has its own directory
            Ok((
                source_path.join("must-gather/Dockerfile"),
                source_path.clone(),
            ))
        }
        "bundle" => {
            // Bundle uses bundle.developer.Dockerfile in project root
            Ok((
                source_path.join("bundle.developer.Dockerfile"),
                source_path.clone(),
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
    build_args: &[(String, String)],
) -> Result<()> {
    use std::process::Stdio;

    let runtime_cmd = runtime.command();

    let mut cmd = Command::new(runtime_cmd);
    cmd.args(["build", "-f"]).arg(dockerfile).args(["-t", tag]);

    // Add build arguments
    for (key, value) in build_args {
        cmd.arg("--build-arg");
        cmd.arg(format!("{}={}", key, value));
    }

    cmd.arg(context)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute {} build command", runtime_cmd))?;

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

    let runtime_cmd = runtime.command();

    let mut cmd = Command::new(runtime_cmd);
    cmd.args(["push", tag])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute {} push command", runtime_cmd))?;

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
