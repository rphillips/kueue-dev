# Quick Start

This guide will walk you through your first kueue-dev workflow in under 5 minutes.

## Prerequisites

Before starting, ensure you have:

- kueue-dev installed ([Installation Guide](./installation.md))
- Docker or Podman running
- kubectl and kind installed

Verify prerequisites:

```bash
kueue-dev check --kind
```

## Step 1: Create a Kind Cluster

Create a new Kubernetes cluster using kind with Calico CNI:

```bash
kueue-dev cluster create --name my-cluster --cni calico
```

You'll see output like:

```
Creating kind cluster 'my-cluster' with calico CNI...
Creating cluster "my-cluster" ...
 ✓ Ensuring node image (kindest/node:v1.30.0) 🖼
 ✓ Preparing nodes 📦 📦 📦 📦
 ✓ Writing configuration 📜
 ✓ Starting control-plane 🕹️
 ✓ Installing CNI 🔌
 ✓ Installing StorageClass 💾
 ✓ Joining worker nodes 🚜
Set kubectl context to "kind-my-cluster"
Installing Calico CNI v3.28.2...
⠋ Downloading Calico manifest...
✓ Calico manifest applied
⠋ Waiting for Calico pods to be ready...
✓ Calico installed successfully
⠋ Labeling worker nodes...
✓ Worker nodes labeled

Cluster 'my-cluster' created successfully!
Kubeconfig: /home/user/.kube/config-my-cluster
```

This creates a 4-node cluster (1 control-plane + 3 workers) with Calico networking.

## Step 2: Build Images (Optional)

If you're building from source, build and push the images:

```bash
kueue-dev images build --related-images dev-images.json
```

This will:
1. Build operator, operand, and must-gather images
2. Push them to your configured registry

> **Note**: Skip this step if using pre-built images from a registry.

## Step 3: Deploy the Operator

Deploy kueue-operator to your cluster:

```bash
kueue-dev deploy kind --name my-cluster --related-images related_images.json
```

> **Note**: Replace `related_images.json` with your images file path.

The deployment process:

1. ✅ Sets kubeconfig context
2. ✅ Loads images to kind cluster
3. ✅ Installs cert-manager
4. ✅ Installs JobSet and LeaderWorkerSet
5. ✅ Installs operator CRDs
6. ✅ Deploys the operator
7. ✅ Waits for operator to be ready

Output:

```
Deploying kueue-operator to kind cluster 'my-cluster'...
Using images from: related_images.json

Images to be used:
  Operator:     quay.io/openshift/kueue-operator:latest
  Operand:      quay.io/openshift/kueue:latest
  Must-gather:  quay.io/openshift/kueue-must-gather:latest

⠋ Loading images to cluster...
✓ All images loaded

⠋ Installing cert-manager v1.13.3...
✓ cert-manager installed

⠋ Installing JobSet v0.10.1...
✓ JobSet installed

⠋ Installing LeaderWorkerSet v0.7.0...
✓ LeaderWorkerSet installed

⠋ Installing operator CRDs...
✓ CRDs installed

⠋ Deploying operator...
✓ Operator deployed

⠋ Waiting for operator to be ready...
✓ Operator ready

==========================================
Deployment completed successfully!
==========================================

To view operator logs:
  kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator -f \
    --kubeconfig=/home/user/.kube/config-my-cluster
```

**Customize Kueue CR (Optional):**

You can customize which frameworks are enabled or skip CR creation entirely:

```bash
# Deploy with only specific frameworks
kueue-dev deploy kind --name my-cluster \
  --kueue-frameworks BatchJob,Pod,JobSet

# Use a custom namespace
kueue-dev deploy kind --name my-cluster \
  --kueue-namespace my-namespace

# Skip Kueue CR creation (deploy operator only, create CR manually later)
kueue-dev deploy kind --name my-cluster \
  --skip-kueue-cr
```

See [Configuration](./configuration.md) for more options.

## Step 4: Run Tests

Run the e2e test suite:

```bash
kueue-dev test run --kubeconfig ~/.kube/config-my-cluster
```

Or with focus on specific tests:

```bash
kueue-dev test run --focus "webhook" --kubeconfig ~/.kube/config-my-cluster
```

Or with a specific label filter:

```bash
kueue-dev test run --label-filter "network-policy" --kubeconfig ~/.kube/config-my-cluster
```

> **Note**: Label filters use Ginkgo's label filtering syntax. The default is `!disruptive` which excludes disruptive tests. You can use expressions like `"network-policy"` to run only tests with that label, or `"!slow"` to exclude slow tests.

Output:

```
Running e2e tests...
⠋ Installing Ginkgo v2.1.4...
✓ Ginkgo installed

Running test suite...
Running Suite: E2E Test Suite
==============================
Random Seed: 1234

Will run 45 of 120 specs
⠋ Running tests...
••••••••••••••••••••••••••••••••••••••••••••••••

Ran 45 of 120 Specs in 180.234 seconds
SUCCESS! -- 45 Passed | 0 Failed | 0 Pending | 75 Skipped

All tests passed!
```

## Step 3.5: Running Upstream Tests (OpenShift Only)

If you have an OpenShift cluster and want to run the upstream kueue tests, you can use the `test upstream` command:

```bash
kueue-dev test upstream --kubeconfig /path/to/openshift-kubeconfig
```

This command will:
1. Apply necessary git patches to the upstream kueue source
2. Label worker nodes for e2e testing
3. Configure OpenShift Security Context Constraints (SCC)
4. Run the upstream kueue test suite

Options:

```bash
# Run with focus pattern
kueue-dev test upstream --focus "webhook"

# Run with label filter
kueue-dev test upstream --label-filter "!disruptive"

# Run tests from specific target folder (default: singlecluster)
kueue-dev test upstream --target singlecluster

# Combine multiple options
kueue-dev test upstream --focus "webhook" --label-filter "!slow" --target singlecluster
```

> **Note**: The `test upstream` command requires:
> - An OpenShift cluster with `oc` CLI configured
> - The upstream kueue source code in `../upstream/kueue/src`
> - Cluster admin permissions to configure SCC policies

Output:

```
Running upstream kueue tests...
⠋ Applying git patches...
✓ Applied patch: e2e.patch
✓ Applied patch: golang-1.24.patch

⠋ Labeling worker nodes...
✓ Labeled node-1 as on-demand
✓ Labeled node-2 as spot

⠋ Configuring OpenShift SCC...
✓ Added privileged SCC
✓ Added anyuid SCC

⠋ Running upstream e2e tests...
Running Suite: Kueue E2E Test Suite
===================================
Random Seed: 5678

Will run 250 of 350 specs
⠋ Running tests...

Ran 250 of 350 Specs in 450.123 seconds
SUCCESS! -- 250 Passed | 0 Failed | 0 Pending | 100 Skipped

==========================================
Upstream e2e tests passed successfully!
==========================================
```

## Step 5: Interactive Debugging (Optional)

Launch the interactive debugging menu:

```bash
kueue-dev interactive --kubeconfig ~/.kube/config-my-cluster
```

Menu options:

```
==========================================
Interactive Menu
==========================================

Available actions:
  1) Port-forward to Prometheus UI (http://localhost:9090)
  2) View Prometheus Operator logs
  3) View Prometheus instance logs
  4) View Kueue Operator logs
  5) Show cluster information
  6) kubectl shell (interactive)
  7) Exit

Select an action [1-7]:
```

Select **4** to view operator logs in real-time.

## Step 6: Cleanup

When you're done testing:

```bash
# Clean up test resources
kueue-dev cleanup --kubeconfig ~/.kube/config-my-cluster

# Delete the cluster
kueue-dev cluster delete --name my-cluster
```

Output:

```
Cleaning up test resources...
✓ Removed finalizers from resources
✓ Deleted test namespaces
✓ Deleted test PriorityClasses
✓ Cleanup completed

Deleting kind cluster 'my-cluster'...
Deleting cluster "my-cluster" ...
✓ Cluster deleted
✓ Kubeconfig removed
```

## Quick Reference Card

Here's a cheat sheet for common operations:

```bash
# Create cluster
kueue-dev cluster create --name <name> [--cni calico|default]

# Deploy operator
kueue-dev deploy kind --name <name> --related-images <file>

# Run tests
kueue-dev test run [--focus <pattern>] [--label-filter <filter>]

# View logs
kueue-dev interactive

# Cleanup
kueue-dev cleanup
kueue-dev cluster delete --name <name>

# List clusters
kueue-dev cluster list

# Get help
kueue-dev --help
kueue-dev <command> --help
```

## What's Next?

Now that you've completed the basic workflow, explore these topics:

### Learn More

- [Common Workflows](./workflows.md) - Real-world development patterns
- [Command Reference](./commands/cluster.md) - Detailed command documentation
- [Advanced Features](./advanced/verbosity.md) - Dry-run, verbosity, completions

### Customize

- [Configuration](./configuration.md) - Set defaults with config files
- [Custom Images](./commands/images.md) - Use your own operator images

### Troubleshoot

- [Troubleshooting](./troubleshooting/common-issues.md) - Solutions to common problems
- [FAQ](./faq.md) - Frequently asked questions

## Tips for Success

### Use Configuration Files

Save time with a `.kueue-dev.toml`:

```toml
[defaults]
cluster_name = "dev"
images_file = "my-images.json"
```

Then simply run:

```bash
kueue-dev cluster create  # Uses "dev" as name
kueue-dev deploy kind     # Uses "my-images.json"
```

### Enable Verbose Output

For debugging, use `-v` flags:

```bash
kueue-dev -v deploy kind --name test    # Info level
kueue-dev -vv deploy kind --name test   # Debug level
kueue-dev -vvv deploy kind --name test  # Trace level
```

### Use Dry-Run

Preview operations before executing:

```bash
kueue-dev --dry-run deploy kind --name test --related-images related_images.json
```

### Tab Completion

Set up shell completion for faster command entry:

```bash
# Bash
kueue-dev completion bash > ~/.local/share/bash-completion/completions/kueue-dev

# Zsh
kueue-dev completion zsh > ~/.zsh/completions/_kueue-dev

# Fish
kueue-dev completion fish > ~/.config/fish/completions/kueue-dev.fish
```

Happy developing! 🚀
