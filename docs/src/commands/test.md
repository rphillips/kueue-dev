# Test Commands

Documentation for `kueue-dev test` commands.

## Overview

The `kueue-dev test` command provides comprehensive testing capabilities for the kueue-operator. It supports running tests on existing clusters, deploying and testing on new clusters, and running upstream Kueue tests.

## Subcommands

### `test run`

Run e2e tests on an existing cluster with retry capability.

**Usage:**
```bash
kueue-dev test run [OPTIONS]
```

**Options:**
- `-f, --focus <FOCUS>` - Test focus pattern (regex)
- `-l, --label-filter <LABEL_FILTER>` - Label filter for tests (e.g., "!disruptive", "network-policy")
- `-k, --kubeconfig <KUBECONFIG>` - Path to kubeconfig file (or use KUBECONFIG env var)

**Examples:**
```bash
# Run all non-disruptive tests
kueue-dev test run

# Run tests with focus pattern
kueue-dev test run --focus "webhook"

# Run tests with label filter
kueue-dev test run --label-filter "network-policy"

# Run tests with custom kubeconfig
kueue-dev test run --kubeconfig /path/to/kubeconfig
```

### `test operator`

Deploy the operator and run tests. Supports three cluster types: `kubeconfig` (default), `kind`, and `openshift`.

**Usage:**
```bash
kueue-dev test operator [OPTIONS]
```

**Options:**
- `-t, --type <TYPE>` - Type of cluster: `kubeconfig`, `kind`, or `openshift` (default: `kubeconfig`)
- `-n, --name <NAME>` - Cluster name (kind only, default: `kueue-test`)
- `-f, --focus <FOCUS>` - Test focus pattern (regex)
- `-l, --label-filter <LABEL_FILTER>` - Label filter for tests
- `-k, --kubeconfig <KUBECONFIG>` - Path to kubeconfig (kubeconfig type only)
- `--related-images <IMAGES>` - Path to related images JSON file (kind only, default: `related_images.json`)
- `--skip-kueue-cr` - Skip creating Kueue CR (only deploy operator)
- `--kueue-frameworks <FRAMEWORKS>` - Kueue frameworks to enable (comma-separated)
- `--kueue-namespace <NAMESPACE>` - Kueue CR namespace (default: `openshift-kueue-operator`)

**Examples:**

#### Using kubeconfig type (default)
```bash
# Run tests using KUBECONFIG environment variable
kueue-dev test operator

# Run tests with specific kubeconfig file
kueue-dev test operator --kubeconfig ~/.kube/my-cluster-config

# Run tests with focus pattern
kueue-dev test operator --focus "webhook"

# Run tests with label filter
kueue-dev test operator --label-filter "!disruptive"

# Explicitly specify kubeconfig type
kueue-dev test operator --type kubeconfig --kubeconfig /path/to/kubeconfig
```

#### Using kind type
```bash
# Deploy to kind cluster and run tests
kueue-dev test operator --type kind --name my-cluster

# Deploy with focus pattern
kueue-dev test operator --type kind --name my-cluster --focus "webhook"

# Deploy without creating Kueue CR
kueue-dev test operator --type kind --name my-cluster --skip-kueue-cr

# Deploy with specific frameworks
kueue-dev test operator --type kind --name my-cluster --kueue-frameworks BatchJob,Pod,JobSet

# Deploy with custom namespace
kueue-dev test operator --type kind --name my-cluster --kueue-namespace my-namespace
```

#### Using openshift type
```bash
# Run tests on OpenShift cluster (uses oc login context)
kueue-dev test operator --type openshift

# Run tests with focus
kueue-dev test operator --type openshift --focus "webhook"

# Run tests with label filter
kueue-dev test operator --type openshift --label-filter "!disruptive"
```

### `test upstream`

Run upstream kueue tests (requires OpenShift cluster or Kind cluster).

**Usage:**
```bash
kueue-dev test upstream [OPTIONS]
```

**Options:**
- `-f, --focus <FOCUS>` - Test focus pattern (regex)
- `-l, --label-filter <LABEL_FILTER>` - Label filter for tests
- `-k, --kubeconfig <KUBECONFIG>` - Path to kubeconfig file
- `--target <TARGET>` - E2E target folder (default: `singlecluster`)

**Examples:**
```bash
# Run upstream tests
kueue-dev test upstream

# Run with focus pattern
kueue-dev test upstream --focus "webhook"

# Run with label filter
kueue-dev test upstream --label-filter "!disruptive"

# Run with custom target folder
kueue-dev test upstream --target multicluster

# Run with custom kubeconfig
kueue-dev test upstream --kubeconfig /path/to/kubeconfig
```

**Kind Cluster Behavior:**

When running upstream tests on a Kind cluster, the following actions are automatically performed:

1. **Operator Scale Down** - The `openshift-kueue-operator` deployment is scaled to 0 replicas and the command waits for all operator pods to terminate before proceeding
2. **NetworkPolicy Removal** - All NetworkPolicies are deleted from the cluster to avoid networking interference with upstream tests

This ensures that upstream tests run in a clean environment without conflicts from the operator or network policies.

## Cluster Type Comparison

| Type | Description | Use Case |
|------|-------------|----------|
| `kubeconfig` | Use existing cluster via kubeconfig | Testing against any pre-configured cluster |
| `kind` | Create new kind cluster | Local development, creates cluster from scratch |
| `openshift` | Use current oc login context | Testing on OpenShift clusters |

## Test Filtering

### Focus Patterns

The `--focus` flag accepts regex patterns to run specific tests:

```bash
# Run only webhook tests
kueue-dev test operator --focus "webhook"

# Run tests matching multiple patterns
kueue-dev test operator --focus "webhook|admission"
```

### Label Filters

The `--label-filter` flag filters tests by labels:

```bash
# Exclude disruptive tests (default for test run)
kueue-dev test operator --label-filter "!disruptive"

# Run only network-policy tests
kueue-dev test operator --label-filter "network-policy"

# Combine filters
kueue-dev test operator --label-filter "!disruptive && network-policy"
```

## Retry Behavior

The `test run` and `test operator` commands include automatic retry on failure:

1. Tests run to completion
2. If tests fail, you're prompted to debug
3. Press RETURN to re-run tests
4. Press Ctrl+C to exit

This is useful for iterative debugging of test failures.

## Configuration

Test behavior can be configured in `~/.config/kueue-dev/config.toml`:

```toml
[tests]
# Test patterns to skip for operator tests
operator_skip_patterns = [
    "AppWrapper",
    "PyTorch",
    "Metrics"
]

# Test patterns to skip for upstream tests
upstream_skip_patterns = [
    "AppWrapper",
    "PyTorch",
    "TrainJob",
    "Kueuectl"
]
```

## Related

- [Quick Start](../quick-start.md)
- [Local Development Workflow](../workflows/local-development.md)
- [CI/CD Integration](../workflows/ci-cd.md)
- [Troubleshooting Tests](../troubleshooting/tests.md)
