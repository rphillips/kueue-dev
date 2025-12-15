# Operator Deployment

Deploy the kueue-operator (OpenShift operator) to kind or OpenShift clusters.

## Overview

The `kueue-dev deploy operator` commands deploy the kueue-operator, which manages Kueue as an operand. This is the recommended approach for OpenShift environments and production deployments.

## Subcommands

### deploy operator kind

Deploy kueue-operator to a kind cluster using OLM bundle (default) or direct manifests.

```bash
kueue-dev deploy operator kind [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --name <NAME>` | Cluster name | `kueue-test` |
| `--related-images <FILE>` | Path to related images JSON file | `related_images.json` |
| `-k, --kubeconfig <FILE>` | Path to kubeconfig file | Auto-detected |
| `--skip-tests` | Skip tests after deployment | false |
| `--skip-kueue-cr` | Skip creating Kueue CR (only deploy operator) | false |
| `--kueue-frameworks <FRAMEWORKS>` | Comma-separated list of frameworks to enable | All |
| `--kueue-namespace <NAMESPACE>` | Kueue CR namespace | `openshift-kueue-operator` |
| `--no-bundle` | Deploy without OLM bundle (use direct manifests) | false |
| `--cert-manager-version <VERSION>` | Override cert-manager version | From config |
| `--jobset-version <VERSION>` | Override JobSet version | From config |
| `--leaderworkerset-version <VERSION>` | Override LeaderWorkerSet version | From config |
| `--prometheus-version <VERSION>` | Override Prometheus Operator version | From config |

**Examples:**

```bash
# Deploy with OLM bundle (default)
kueue-dev deploy operator kind

# Deploy without bundle (direct manifests)
kueue-dev deploy operator kind --no-bundle

# Deploy to specific cluster with custom images
kueue-dev deploy operator kind --name dev --related-images dev-images.json

# Deploy without creating Kueue CR
kueue-dev deploy operator kind --skip-kueue-cr

# Deploy with specific frameworks enabled
kueue-dev deploy operator kind --kueue-frameworks BatchJob,Pod,JobSet

# Deploy with custom dependency versions
kueue-dev deploy operator kind --cert-manager-version v1.17.0 --jobset-version v0.9.0
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

### deploy operator olm

Deploy via OLM bundle with explicit bundle image.

```bash
kueue-dev deploy operator olm [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-b, --bundle <IMAGE>` | Bundle image (required) | - |
| `-n, --name <NAME>` | Cluster name | `kueue-test` |

**Examples:**

```bash
# Deploy with specific bundle image
kueue-dev deploy operator olm --bundle quay.io/myuser/kueue-bundle:latest

# Deploy to specific cluster
kueue-dev deploy operator olm --bundle quay.io/myuser/kueue-bundle:v0.1.0 --name dev
```

### deploy operator openshift

Deploy to OpenShift cluster.

```bash
kueue-dev deploy operator openshift [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--related-images <FILE>` | Path to related images JSON file | `related_images.json` |
| `--skip-tests` | Skip tests after deployment | false |

**Examples:**

```bash
# Deploy to current OpenShift context
kueue-dev deploy operator openshift

# Deploy with custom images
kueue-dev deploy operator openshift --related-images prod-images.json
```

## Common Workflows

### Quick Local Development

```bash
# Create cluster and deploy in one command
kueue-dev deploy operator kind --name dev

# Or deploy to existing cluster without OLM
kueue-dev deploy operator kind --name dev --no-bundle
```

### Deploy with Custom Configuration

```bash
# Deploy with specific frameworks
kueue-dev deploy operator kind \
  --name dev \
  --kueue-frameworks BatchJob,Pod,Deployment,StatefulSet \
  --kueue-namespace kueue-system
```

### Testing Bundle Before Production

```bash
# Build bundle
kueue-dev images build bundle

# Deploy with bundle to test
kueue-dev deploy operator kind --name bundle-test
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

- [Upstream Deployment](./deploy-upstream.md)
- [Quick Start](../quick-start.md)
- [OLM Workflow](../workflows/olm.md)
