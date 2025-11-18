# Configuration

kueue-dev supports configuration files to set defaults and customize behavior.

## Configuration File

Create a configuration file to avoid repeating common flags:

### Location

Configuration files are loaded in this order (first found wins):

1. **`.kueue-dev.toml`** - Project-specific (current directory)
2. **`~/.config/kueue-dev/config.toml`** - Global user configuration

### Format

Configuration uses TOML format with five main sections:

```toml
[defaults]
cluster_name = "kueue-test"
cni_provider = "calico"
images_file = "related_images.json"

[colors]
enabled = true
theme = "default"

[behavior]
confirm_destructive = true
parallel_operations = true
show_progress = true

[kueue]
namespace = "openshift-kueue-operator"
frameworks = ["BatchJob", "Pod", "Deployment", "StatefulSet", "JobSet", "LeaderWorkerSet"]

[tests]
operator_skip_patterns = ["AppWrapper", "PyTorch", "Metrics", ...]
upstream_skip_patterns = ["AppWrapper", "PyTorch", "TrainJob", "Kueuectl", ...]
```

## Configuration Sections

### [defaults]

Set default values for command-line flags:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `cluster_name` | string | `"kueue-test"` | Default cluster name for commands |
| `cni_provider` | string | `"calico"` | CNI to use: `"calico"` or `"default"` |
| `images_file` | string | `"related_images.json"` | Default images configuration file |

**Example:**

```toml
[defaults]
cluster_name = "my-dev-cluster"
cni_provider = "calico"
images_file = "my-images.json"
```

With this configuration:

```bash
# This command:
kueue-dev cluster create

# Is equivalent to:
kueue-dev cluster create --name my-dev-cluster --cni calico
```

### [colors]

Control terminal color output:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable/disable colored output |
| `theme` | string | `"default"` | Color theme (future use) |

**Example:**

```toml
[colors]
enabled = false  # Disable colors for CI/CD
theme = "default"
```

### [behavior]

Configure tool behavior:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `confirm_destructive` | boolean | `true` | Prompt before destructive operations |
| `parallel_operations` | boolean | `true` | Enable parallel execution |
| `show_progress` | boolean | `true` | Show progress indicators |

**Example:**

```toml
[behavior]
confirm_destructive = true   # Always ask before deletion
parallel_operations = true   # Use parallel operations
show_progress = true         # Show spinners and progress bars
```

**Note:** The `confirm_destructive` setting affects destructive operations like cluster deletion. When set to `false`, confirmations are skipped. You can also override this per-command using the `--force` flag:

```bash
# Skip confirmation for this deletion only
kueue-dev cluster delete --name test --force
```

### [kueue]

Configure the Kueue Custom Resource (CR) that will be created during deployment:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `namespace` | string | `"openshift-kueue-operator"` | Kueue CR namespace |
| `frameworks` | array | See below | List of frameworks to enable |

**Default frameworks:**
- `BatchJob` - Kubernetes Batch Jobs
- `Pod` - Kubernetes Pods
- `Deployment` - Kubernetes Deployments
- `StatefulSet` - Kubernetes StatefulSets
- `JobSet` - JobSet framework
- `LeaderWorkerSet` - LeaderWorkerSet framework

**Example:**

```toml
[kueue]
# Enable only specific frameworks
frameworks = ["BatchJob", "Pod", "JobSet"]
```

**Note:** The `namespace` value should typically remain at its default (`"openshift-kueue-operator"`) as this is the standard value expected by the kueue-operator. The Kueue CR name is always "cluster" and is not configurable.

**Command-line override:**

You can override the frameworks and namespace at deployment time, or skip CR creation entirely:

```bash
# Enable only specific frameworks for this deployment
kueue-dev deploy kind --kueue-frameworks BatchJob,Pod,JobSet

# Override the namespace
kueue-dev deploy kind --kueue-namespace my-kueue-namespace

# Skip Kueue CR creation (deploy operator only)
kueue-dev deploy kind --skip-kueue-cr

# Combine multiple overrides
kueue-dev deploy kind --kueue-frameworks BatchJob,Pod --kueue-namespace my-namespace

# Or when running tests
kueue-dev test operator --type kind --kueue-frameworks BatchJob,Pod --kueue-namespace my-namespace

# Skip CR creation during tests
kueue-dev test operator --type kind --skip-kueue-cr
```

The command-line options take precedence over the configuration file.

**Use cases for `--skip-kueue-cr`:**
- Testing operator deployment without deploying kueue components
- Manually creating/customizing the CR after deployment
- Advanced debugging scenarios where you want to deploy just the operator infrastructure first

### [tests]

Configure test skip patterns for both operator and upstream tests:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `operator_skip_patterns` | array | See below | Test patterns to skip for operator tests |
| `upstream_skip_patterns` | array | See below | Test patterns to skip for upstream tests |

**Default operator skip patterns:**
- `AppWrapper`, `PyTorch`, `JobSet`, `LeaderWorkerSet`
- `JAX`, `Kuberay`, `Metrics`, `Fair`
- `TopologyAwareScheduling`, `Kueue visibility server`
- CPU-dependent tests: `Failed Pod can be replaced in group`, `should allow to schedule a group of diverse pods`, `StatefulSet created with WorkloadPriorityClass`

**Default upstream skip patterns:**
- Same as operator patterns, plus:
- `TrainJob` (instead of JobSet)
- `Kueuectl` (kueuectl not included in operator)

**Example:**

```toml
[tests]
# Custom skip patterns for operator tests
operator_skip_patterns = [
    "AppWrapper",
    "PyTorch",
    "Metrics"
]

# Custom skip patterns for upstream tests
upstream_skip_patterns = [
    "AppWrapper",
    "PyTorch",
    "Metrics",
    "Kueuectl"
]
```

**Use cases:**
- Customize which tests to skip based on your cluster capabilities
- Enable tests that are disabled by default once the cluster supports them
- Skip additional tests that are flaky or not relevant to your environment

**Note:** Test skip patterns use Ginkgo's skip pattern syntax and are combined into a regex like `(pattern1|pattern2|pattern3)`.

## Example Configurations

### Minimal Configuration

```toml
[defaults]
cluster_name = "dev"
```

### Development Configuration

```toml
# .kueue-dev.toml (in project root)
[defaults]
cluster_name = "kueue-dev"
cni_provider = "calico"
images_file = "dev-images.json"

[behavior]
confirm_destructive = false  # Skip confirmations for faster iteration
show_progress = true
```

### CI/CD Configuration

```toml
# ~/.config/kueue-dev/config.toml (on CI runner)
[defaults]
cluster_name = "ci-test"

[colors]
enabled = false  # No colors in CI logs

[behavior]
confirm_destructive = false  # No interactive prompts
show_progress = false        # No progress bars in CI
```

### Multi-Environment Setup

For developers working with multiple environments:

```toml
# ~/.config/kueue-dev/config.toml (global defaults)
[defaults]
cni_provider = "calico"

[colors]
enabled = true

[behavior]
confirm_destructive = true
show_progress = true
```

Then use project-specific configs:

```toml
# ~/projects/feature-a/.kueue-dev.toml
[defaults]
cluster_name = "feature-a"
images_file = "feature-a-images.json"
```

```toml
# ~/projects/bugfix-123/.kueue-dev.toml
[defaults]
cluster_name = "bugfix-123"
images_file = "bugfix-images.json"
```

## Command-Line Override

Command-line flags always override configuration file values:

```bash
# Even with cluster_name = "dev" in config:
kueue-dev cluster create --name prod-test

# This creates a cluster named "prod-test"
```

## Viewing Active Configuration

To see what configuration kueue-dev would use:

```bash
# Check if config exists
test -f .kueue-dev.toml && echo "Project config found"
test -f ~/.config/kueue-dev/config.toml && echo "Global config found"

# View config
cat .kueue-dev.toml
# or
cat ~/.config/kueue-dev/config.toml
```

## Configuration Tips

### Per-Feature Development

Create a config per feature branch:

```bash
# Switch to feature branch
git checkout feature/new-webhook

# Create branch-specific config
cat > .kueue-dev.toml <<EOF
[defaults]
cluster_name = "webhook-dev"
images_file = "webhook-test-images.json"
EOF

# Now all commands use these defaults
kueue-dev cluster create  # Creates "webhook-dev" cluster
```

### Team Standards

Share a recommended config in the repository:

```bash
# .kueue-dev.toml.example
[defaults]
cni_provider = "calico"

[behavior]
show_progress = true
```

Team members copy to `.kueue-dev.toml`:

```bash
cp .kueue-dev.toml.example .kueue-dev.toml
```

Add `.kueue-dev.toml` to `.gitignore` for personal customization.

## Troubleshooting

### Config Not Loading

If your config isn't being used:

1. **Check file location**: Must be `.kueue-dev.toml` (note the dot)
2. **Check file syntax**: TOML syntax must be valid
3. **Check file permissions**: File must be readable

Validate TOML syntax:

```bash
# Install toml-cli if needed
cargo install toml-cli

# Validate config
toml get .kueue-dev.toml defaults
```

### Syntax Errors

Common TOML mistakes:

```toml
# ❌ Wrong: quotes around boolean
confirm_destructive = "true"

# ✅ Correct: boolean without quotes
confirm_destructive = true

# ❌ Wrong: missing quotes around string
cluster_name = dev

# ✅ Correct: string with quotes
cluster_name = "dev"
```

## Next Steps

- [Quick Start](./quick-start.md) to create your first cluster
- [Common Workflows](./workflows.md) for real-world usage patterns
- [Command Reference](./commands/cluster.md) for detailed command documentation
