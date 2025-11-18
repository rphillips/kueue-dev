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

See specific troubleshooting guides:
- [Cluster Connection](./cluster-connection.md)
- [Deployment Failures](./deployment.md)
- [Test Failures](./tests.md)
