# Cluster Commands

Documentation for `kueue-dev cluster` commands.

## Overview

The `cluster` commands manage kind (Kubernetes in Docker) clusters for local development and testing.

## Commands

### create

Create a new kind cluster for kueue-operator development.

```bash
kueue-dev cluster create [OPTIONS]
```

**Options:**

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--name` | `-n` | Cluster name | `"kueue-test"` (or from config) |
| `--cni` | | CNI provider: `calico` or `default` | `"calico"` (or from config) |
| `--kubeconfig` | `-k` | Path to save kubeconfig file | None (uses kind default) |

**Examples:**

```bash
# Create cluster with defaults (uses config file settings)
kueue-dev cluster create

# Create cluster with custom name
kueue-dev cluster create --name my-cluster

# Create cluster with specific CNI
kueue-dev cluster create --cni default

# Create cluster and save kubeconfig to specific location
kueue-dev cluster create --name dev --kubeconfig ./kubeconfig

# Override config file defaults
kueue-dev cluster create --name test --cni calico
```

**CNI Provider:**

The `--cni` flag is optional and defaults to the value in your configuration file (default: `"calico"`). Calico is recommended for most development scenarios as it provides better network policy support.

If you don't specify `--cni` and don't have a config file, it will use `"calico"`.

### delete

Delete an existing kind cluster.

```bash
kueue-dev cluster delete [OPTIONS]
```

**Options:**

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--name` | `-n` | Cluster name | `"kueue-test"` (or from config) |
| `--force` | `-f` | Skip confirmation prompt | `false` |

**Examples:**

```bash
# Delete cluster (will prompt for confirmation)
kueue-dev cluster delete --name my-cluster

# Delete cluster without confirmation
kueue-dev cluster delete --name my-cluster --force

# Delete default cluster
kueue-dev cluster delete
```

### list

List all kind clusters on the system.

```bash
kueue-dev cluster list
```

**Examples:**

```bash
# List all kind clusters
kueue-dev cluster list
```

## Configuration

The cluster commands use configuration from `.kueue-dev.toml`:

```toml
[defaults]
cluster_name = "kueue-test"
cni_provider = "calico"
```

See [Configuration](../configuration.md) for more details.

## Related

- [Quick Start](../quick-start.md)
- [Configuration](../configuration.md)
- [Deploy Commands](deploy.md)
