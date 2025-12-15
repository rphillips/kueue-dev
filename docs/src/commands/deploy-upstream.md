# Upstream Deployment

Deploy upstream Kueue (vanilla Kubernetes) using kustomize or helm from a source tree.

## Overview

The `kueue-dev deploy upstream` commands deploy upstream Kueue directly from source, without the operator. This is useful for:
- Testing upstream Kueue changes
- Development against vanilla Kubernetes
- Environments where the operator is not needed

## Prerequisites

- **kustomize** (for kustomize deployment): Install from https://kubectl.docs.kubernetes.io/installation/kustomize/
- **helm** (for helm deployment): Install from https://helm.sh/docs/intro/install/
- **make** (for building images): Required when using `--build-image`

Run `kueue-dev check` to verify prerequisites are installed.

## Subcommands

### deploy upstream kustomize

Deploy upstream Kueue using kustomize overlays.

```bash
kueue-dev deploy upstream kustomize [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--upstream-source <PATH>` | Path to upstream kueue source directory | Auto-detected or from config |
| `-o, --overlay <NAME>` | Kustomize overlay to use (default, dev, alpha-enabled) | `default` |
| `--image <IMAGE>` | Override controller image | From overlay |
| `--build-image` | Build kueue image from source and load to kind | false |
| `--image-tag <TAG>` | Custom image tag when building | `localhost/kueue:dev` |
| `-n, --namespace <NS>` | Namespace to deploy to | `kueue-system` |
| `-c, --cluster-name <NAME>` | Cluster name (for kind clusters) | `kueue-test` |
| `-k, --kubeconfig <FILE>` | Path to kubeconfig file | Auto-detected |
| `--skip-deps` | Skip installing dependencies | false |
| `--cert-manager-version <VERSION>` | Override cert-manager version | From config |
| `--jobset-version <VERSION>` | Override JobSet version | From config |
| `--leaderworkerset-version <VERSION>` | Override LeaderWorkerSet version | From config |

**Examples:**

```bash
# Deploy from source with default overlay
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src

# Deploy with dev overlay
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src --overlay dev

# Deploy with custom image
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src \
  --image gcr.io/my-project/kueue:dev

# Build image from source and deploy
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src --build-image

# Build with custom image tag
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src \
  --build-image --image-tag my-registry/kueue:test

# Skip dependency installation (if already installed)
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src --skip-deps
```

### deploy upstream helm

Deploy upstream Kueue using the helm chart.

```bash
kueue-dev deploy upstream helm [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--upstream-source <PATH>` | Path to upstream kueue source directory | Auto-detected or from config |
| `-r, --release-name <NAME>` | Helm release name | `kueue` |
| `-n, --namespace <NS>` | Namespace to deploy to | `kueue-system` |
| `-f, --values-file <FILE>` | Path to values.yaml override file | None |
| `--set <KEY=VALUE>` | Set helm values (can be repeated) | None |
| `--build-image` | Build kueue image from source and load to kind | false |
| `--image-tag <TAG>` | Custom image tag when building | `localhost/kueue:dev` |
| `-c, --cluster-name <NAME>` | Cluster name (for kind clusters) | `kueue-test` |
| `-k, --kubeconfig <FILE>` | Path to kubeconfig file | Auto-detected |
| `--skip-deps` | Skip installing dependencies | false |
| `--cert-manager-version <VERSION>` | Override cert-manager version | From config |
| `--jobset-version <VERSION>` | Override JobSet version | From config |
| `--leaderworkerset-version <VERSION>` | Override LeaderWorkerSet version | From config |

**Examples:**

```bash
# Deploy with helm defaults
kueue-dev deploy upstream helm --upstream-source /path/to/kueue/src

# Deploy with custom values file
kueue-dev deploy upstream helm --upstream-source /path/to/kueue/src \
  -f my-values.yaml

# Deploy with inline value overrides
kueue-dev deploy upstream helm --upstream-source /path/to/kueue/src \
  --set controllerManager.replicas=2 \
  --set controllerManager.manager.image.pullPolicy=Always

# Build image from source and deploy
kueue-dev deploy upstream helm --upstream-source /path/to/kueue/src --build-image

# Deploy with custom release name
kueue-dev deploy upstream helm --upstream-source /path/to/kueue/src \
  --release-name kueue-dev
```

## Source Path Resolution

The upstream source path is resolved in the following order:

1. **CLI flag**: `--upstream-source /path/to/kueue/src`
2. **Environment variable**: `KUEUE_UPSTREAM_SOURCE=/path/to/kueue/src`
3. **Config file**: Set `defaults.upstream_source` in your config
4. **Current directory**: If it contains `config/default/kustomization.yaml` or `charts/kueue/Chart.yaml`

### Configuration Example

Add to `~/.config/kueue-dev/config.toml`:

```toml
[defaults]
upstream_source = "/home/user/kueue/src"
```

## Image Building

When `--build-image` is specified, the tool:

1. Runs `make kind-image-build` in the upstream source directory
2. The image is built and loaded into the local Docker daemon
3. Loads the image to the kind cluster
4. Configures the deployment to use the built image

### Build Requirements

- `make` must be available in PATH
- Docker or Podman for building images
- Go toolchain (for compiling kueue)

### Custom Image Tags

Use `--image-tag` to specify a custom image registry and tag. The upstream Makefile
automatically appends `/kueue` to the registry, so you only need to specify the registry:

```bash
# Default: builds localhost/kueue:dev
kueue-dev deploy upstream kustomize \
  --upstream-source /path/to/kueue/src \
  --build-image

# Custom registry and tag: builds my-registry/kueue:feature-branch
kueue-dev deploy upstream kustomize \
  --upstream-source /path/to/kueue/src \
  --build-image \
  --image-tag my-registry:feature-branch

# You can also include /kueue explicitly (it will be handled correctly)
kueue-dev deploy upstream kustomize \
  --upstream-source /path/to/kueue/src \
  --build-image \
  --image-tag my-registry/kueue:v1.0
```

For helm deployments with `--build-image`, the tool automatically sets:
- `controllerManager.manager.image.repository`
- `controllerManager.manager.image.tag`
- `controllerManager.manager.image.pullPolicy=Never`

## Dependencies

The following dependencies are installed automatically (unless `--skip-deps` is used):

| Dependency | Purpose |
|------------|---------|
| cert-manager | Webhook certificates |
| JobSet | JobSet workload support |
| LeaderWorkerSet | LeaderWorkerSet workload support |

Dependencies are installed in parallel for faster deployment.

## Common Workflows

### Testing Local Changes

```bash
# Make changes to upstream kueue
cd /path/to/kueue/src
# ... edit code ...

# Build and deploy
kueue-dev deploy upstream kustomize --upstream-source . --build-image
```

### Switching Between Overlays

```bash
# Deploy default overlay
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src

# Switch to dev overlay
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src --overlay dev

# Switch to alpha-enabled overlay
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src --overlay alpha-enabled
```

### Using Pre-built Images

```bash
# Use a released image
kueue-dev deploy upstream kustomize \
  --upstream-source /path/to/kueue/src \
  --image gcr.io/k8s-staging-kueue/kueue:main

# Use helm with specific image
kueue-dev deploy upstream helm \
  --upstream-source /path/to/kueue/src \
  --set controllerManager.manager.image.repository=gcr.io/k8s-staging-kueue/kueue \
  --set controllerManager.manager.image.tag=main
```

## Troubleshooting

### Missing kustomize or helm

```
kustomize is required but not found in PATH.
Install from: https://kubectl.docs.kubernetes.io/installation/kustomize/
```

Run `kueue-dev check` to see which tools are missing.

### Source Path Not Found

```
No upstream kueue source specified.
Specify the path with --upstream-source or set defaults.upstream_source in config.
```

Ensure you provide `--upstream-source` or configure it in your config file.

### Image Build Failures

If `--build-image` fails:
1. Ensure you have Docker or Podman running
2. Check that Go is installed and in PATH
3. Verify the source directory has a valid Makefile
4. Check `make kind-image-build` runs manually

## Related

- [Operator Deployment](./deploy-operator.md)
- [Cluster Management](./cluster.md)
- [Configuration](../configuration.md)
