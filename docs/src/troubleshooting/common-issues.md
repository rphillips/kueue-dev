# Common Issues

Solutions to frequently encountered problems.

## Quick Diagnostic

Run verbose mode to see detailed information:

```bash
kueue-dev -vv <command>
```

## Common Error Patterns

### "Command not found"

**Symptom**: `kueue-dev: command not found`

**Solution**:
```bash
# Verify installation
which kueue-dev

# If not found, ensure it's in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Or reinstall
cargo install --path kueue-dev
```

### "Permission denied"

**Symptom**: Various permission errors

**Solution**:
- Check Docker/Podman is running and you have access
- For OpenShift: Verify you have cluster-admin with `oc auth can-i '*' '*' --all-namespaces`
- Check file permissions on kubeconfig

### "Image not found"

**Symptom**: Failed to load images to kind cluster

**Solution**:
```bash
# Verify image exists locally
docker images | grep kueue

# Pull image if needed
docker pull quay.io/openshift/kueue-operator:latest

# Check images file is correct
kueue-dev images list --file related_images.json
```

### "Image 'X' not found in configuration"

**Symptom**: Error message like `Image 'bundle' not found in configuration` when building images

**Cause**: The configuration file (`.kueue-dev.toml`) is not being found, causing the tool to fall back to default settings.

**Solution**:
1. **Verify your configuration file exists** in the current directory or `~/.config/kueue-dev/config.toml`:
   ```bash
   # Check for local config
   ls -la .kueue-dev.toml

   # Check for global config
   ls -la ~/.config/kueue-dev/config.toml
   ```

2. **Check that the images file path is correct** in your config:
   ```toml
   [defaults]
   images_file = "/path/to/related_images.json"
   ```

3. **Verify the images file contains all required images**:
   ```bash
   cat /path/to/related_images.json
   ```

   Make sure it includes entries for all components (operator, operand, must-gather, bundle):
   ```json
   [
     {"name": "operator", "image": "quay.io/user/kueue-operator:latest"},
     {"name": "operand", "image": "quay.io/user/kueue:latest"},
     {"name": "must-gather", "image": "quay.io/user/kueue-must-gather:latest"},
     {"name": "bundle", "image": "quay.io/user/kueue-bundle:latest"}
   ]
   ```

4. **Use an absolute path** for the images file in your config to avoid path resolution issues.

See specific troubleshooting guides:
- [Cluster Connection](./cluster-connection.md)
- [Deployment Failures](./deployment.md)
- [Test Failures](./tests.md)
