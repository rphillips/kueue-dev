# Deploy Commands

Documentation for `kueue-dev deploy` commands.

## Overview

The `kueue-dev deploy` commands provide streamlined deployment workflows for kueue-operator to different environments.

## Subcommands

### deploy kind

Deploy kueue-operator to a kind cluster using OLM bundle (default) or direct manifests.

```bash
kueue-dev deploy kind [OPTIONS]
```

**Options:**
- `-n, --name <NAME>` - Cluster name (default: `kueue-test`)
- `--related-images <FILE>` - Path to related images JSON file
- `-k, --kubeconfig <FILE>` - Path to kubeconfig file
- `--skip-tests` - Skip tests after deployment
- `--skip-kueue-cr` - Skip creating Kueue CR (only deploy operator)
- `--kueue-frameworks <FRAMEWORKS>` - Comma-separated list of frameworks to enable
- `--kueue-namespace <NAMESPACE>` - Kueue CR namespace (default: openshift-kueue-operator)
- `--no-bundle` - Deploy without OLM bundle (use direct manifests)
- `--cert-manager-version <VERSION>` - Override cert-manager version (e.g., v1.18.0)
- `--jobset-version <VERSION>` - Override JobSet version (e.g., v0.10.1)
- `--leaderworkerset-version <VERSION>` - Override LeaderWorkerSet version (e.g., v0.7.0)
- `--prometheus-version <VERSION>` - Override Prometheus Operator version (e.g., v0.82.2)

**Examples:**

```bash
# Deploy with OLM bundle (default)
kueue-dev deploy kind

# Deploy without bundle (direct manifests)
kueue-dev deploy kind --no-bundle

# Deploy to specific cluster with custom images
kueue-dev deploy kind --name dev --related-images dev-images.json

# Deploy without creating Kueue CR
kueue-dev deploy kind --skip-kueue-cr

# Deploy with specific frameworks enabled
kueue-dev deploy kind --kueue-frameworks BatchJob,Pod,JobSet

# Deploy with custom dependency versions
kueue-dev deploy kind --cert-manager-version v1.17.0 --jobset-version v0.9.0
```

**Deployment Methods:**

By default, deployment uses OLM bundle which:
- Installs OLM if not already present
- Deploys operator via `operator-sdk run bundle`
- Provides production-like deployment experience
- Requires `operator-sdk` binary

Use `--no-bundle` flag to deploy via direct manifests which:
- Installs cert-manager, JobSet, LeaderWorkerSet, and Prometheus Operator
- Applies CRDs and operator manifests directly
- Faster for development iteration
- Does not require `operator-sdk`

**Dependencies Installed:**

Both deployment methods install these dependencies in parallel:
- cert-manager (for webhook certificates)
- JobSet (for JobSet workloads)
- LeaderWorkerSet (for LeaderWorkerSet workloads)
- Prometheus Operator (for metrics collection)

**Version Information:**

After deployment, the tool displays:
- Operator version (extracted from pod logs)
- Kueue controller-manager version (if running)

### deploy olm

Deploy via OLM bundle with explicit bundle image.

```bash
kueue-dev deploy olm [OPTIONS]
```

**Options:**
- `-b, --bundle <IMAGE>` - Bundle image
- `-n, --name <NAME>` - Cluster name (default: `kueue-test`)

**Examples:**

```bash
# Deploy with specific bundle image
kueue-dev deploy olm --bundle quay.io/myuser/kueue-bundle:latest

# Deploy to specific cluster
kueue-dev deploy olm --bundle quay.io/myuser/kueue-bundle:v0.1.0 --name dev
```

### deploy openshift

Deploy to OpenShift cluster.

```bash
kueue-dev deploy openshift [OPTIONS]
```

**Options:**
- `--related-images <FILE>` - Path to related images JSON file
- `--skip-tests` - Skip tests after deployment

**Examples:**

```bash
# Deploy to current OpenShift context
kueue-dev deploy openshift

# Deploy with custom images
kueue-dev deploy openshift --related-images prod-images.json
```

## Common Workflows

### Quick Local Development

```bash
# Create cluster and deploy in one command
kueue-dev deploy kind --name dev

# Or deploy to existing cluster
kueue-dev deploy kind --name dev --no-bundle
```

### Deploy with Custom Configuration

```bash
# Deploy with specific frameworks
kueue-dev deploy kind \
  --name dev \
  --kueue-frameworks BatchJob,Pod,Deployment,StatefulSet \
  --kueue-namespace kueue-system
```

### Testing Bundle Before Production

```bash
# Build bundle
kueue-dev images build bundle

# Deploy with bundle to test
kueue-dev deploy kind --name bundle-test
```

## Troubleshooting

### OLM Installation Issues

If OLM installation fails, you can:
1. Use `--no-bundle` to skip OLM
2. Check if OLM is already installed: `kubectl get ns olm`
3. Verify `operator-sdk` is available: `operator-sdk version`

### Missing operator-sdk

If you see an error about missing `operator-sdk`:
```
operator-sdk is required for bundle deployment but not found in PATH.
Install from: https://sdk.operatorframework.io/docs/installation/
Or use --no-bundle to deploy without OLM
```

Solution:
- Install operator-sdk from https://sdk.operatorframework.io/docs/installation/
- Or use `--no-bundle` flag to deploy via direct manifests

## Related

- [Quick Start](../quick-start.md)
- [Common Workflows](../workflows.md)
- [OLM Workflow](../workflows/olm.md)
