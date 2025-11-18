# kueue-dev

Development CLI tool for kueue-operator.

**Documentation:** https://trolocsis.com/kueue-dev/

## Overview

`kueue-dev` is a Rust-based CLI tool that replaces the shell scripts in the kueue-operator project. It provides a unified interface for managing kind clusters, deploying the operator, and running tests.

## Building

```bash
cargo build --release
```

The binary will be available at `target/release/kueue-dev`.

## Installation

```bash
cargo install --path .
```

## Usage

### Check Prerequisites

Check if all required tools are installed:

```bash
# Check basic prerequisites
kueue-dev check

# Check kind-specific prerequisites
kueue-dev check --kind

# Check OpenShift-specific prerequisites
kueue-dev check --openshift

# Check OLM prerequisites
kueue-dev check --olm
```

### Cluster Management ✅ WORKING

```bash
# Create a kind cluster with Calico CNI
kueue-dev cluster create --name my-cluster --cni calico

# Create a kind cluster with default CNI
kueue-dev cluster create --name my-cluster --cni default

# Delete a kind cluster
kueue-dev cluster delete --name my-cluster

# Delete without confirmation prompt (force)
kueue-dev cluster delete --name my-cluster --force

# List kind clusters
kueue-dev cluster list
```

### Deploy ✅ WORKING

```bash
# Deploy to existing kind cluster
kueue-dev deploy kind --name my-cluster --related-images related_images.json

# Deploy with skip tests flag
kueue-dev deploy kind --name my-cluster --skip-tests

# Deploy operator without creating Kueue CR (for manual CR creation)
kueue-dev deploy kind --name my-cluster --skip-kueue-cr

# Deploy with specific Kueue frameworks
kueue-dev deploy kind --name my-cluster --kueue-frameworks BatchJob,Pod,JobSet

# Deploy with custom namespace
kueue-dev deploy kind --name my-cluster --kueue-namespace my-namespace

# Deploy via OLM bundle
kueue-dev deploy olm --bundle quay.io/my-org/bundle:latest --name my-cluster

# Deploy to OpenShift cluster
kueue-dev deploy openshift --related-images related_images.json

# Deploy to OpenShift with skip tests
kueue-dev deploy openshift --related-images related_images.json --skip-tests
```

### Testing ✅ WORKING

```bash
# Run tests on existing cluster (with retry)
kueue-dev test run

# Run tests with focus pattern
kueue-dev test run --focus "webhook"

# Run tests with specific label filter
kueue-dev test run --label-filter "network-policy"

# Run tests using environment KUBECONFIG (default type: kubeconfig)
kueue-dev test operator

# Run tests with specific kubeconfig file
kueue-dev test operator --kubeconfig /path/to/kubeconfig

# Run tests with focus pattern on existing cluster
kueue-dev test operator --focus "webhook"

# Run tests with label filter on existing cluster
kueue-dev test operator --label-filter "!disruptive"

# Deploy operator and run tests on kind cluster
kueue-dev test operator --type kind --name my-cluster --related-images related_images.json

# Deploy operator and run tests with focus
kueue-dev test operator --type kind --name my-cluster --focus "webhook" --related-images related_images.json

# Deploy operator and run tests with label filter
kueue-dev test operator --type kind --name my-cluster --label-filter "!disruptive" --related-images related_images.json

# Deploy operator without creating Kueue CR
kueue-dev test operator --type kind --name my-cluster --skip-kueue-cr

# Deploy operator with specific frameworks
kueue-dev test operator --type kind --name my-cluster --kueue-frameworks BatchJob,Pod,JobSet

# Run tests on OpenShift cluster
kueue-dev test operator --type openshift

# Run tests on OpenShift with focus
kueue-dev test operator --type openshift --focus "webhook"

# Run tests on OpenShift with label filter
kueue-dev test operator --type openshift --label-filter "!disruptive"

# Run upstream kueue tests (requires OpenShift cluster)
kueue-dev test upstream

# Run upstream tests with focus
kueue-dev test upstream --focus "webhook"

# Run upstream tests with label filter
kueue-dev test upstream --label-filter "!disruptive"

# Run upstream tests with specific target folder
kueue-dev test upstream --target singlecluster

# Run upstream tests with custom kubeconfig
kueue-dev test upstream --kubeconfig /path/to/kubeconfig
```

### Cleanup ✅ WORKING

```bash
# Clean up test resources
kueue-dev cleanup

# Clean up with specific kubeconfig
kueue-dev cleanup --kubeconfig /path/to/kubeconfig
```

### Images ✅ WORKING

```bash
# Build and push container images
kueue-dev images build

# Build specific components
kueue-dev images build operator,operand

# Build with custom images file
kueue-dev images build --related-images dev-images.json

# Build all components in parallel with animated spinners (faster)
kueue-dev images build --parallel

# List images from config
kueue-dev images list --file related_images.json

# Load images to kind cluster
kueue-dev images load --name my-cluster --related-images related_images.json
```

**Parallel Build Output:**
```
⠋ operator [3/4] Building image...
⠙ operand [2/4] Locating Dockerfile...
✓ must-gather Complete
```

Features:
- Animated circular spinners for each component
- Color-coded status updates
- Terminal title updates with progress
- Clean, Docker-style output

### Interactive Menu ✅ WORKING

```bash
# Launch interactive debugging menu
kueue-dev interactive

# Launch with custom kubeconfig
kueue-dev interactive --kubeconfig /path/to/kubeconfig
```

Interactive menu provides:
- Port-forward to Prometheus UI (http://localhost:9090)
- View Prometheus Operator logs
- View Prometheus instance logs
- View Kueue Operator logs
- Show cluster information
- Interactive kubectl shell

### Advanced Features ✅ WORKING

```bash
# Generate shell completions
kueue-dev completion bash > /etc/bash_completion.d/kueue-dev
kueue-dev completion zsh > /usr/local/share/zsh/site-functions/_kueue-dev

# Dry-run mode (preview without making changes)
kueue-dev --dry-run deploy kind --name test --related-images related_images.json

# Multiple verbosity levels
kueue-dev -v deploy kind --name test        # Info level
kueue-dev -vv deploy kind --name test       # Debug level
kueue-dev -vvv deploy kind --name test      # Trace level
```

### Configuration File ✅ WORKING

Create `~/.config/kueue-dev/config.toml` or `.kueue-dev.toml` in your project:

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
# Kueue CR namespace (default: "openshift-kueue-operator")
namespace = "openshift-kueue-operator"
# Frameworks to enable
frameworks = ["BatchJob", "Pod", "Deployment", "StatefulSet", "JobSet", "LeaderWorkerSet"]

[tests]
# Test patterns to skip for operator tests
operator_skip_patterns = ["AppWrapper", "PyTorch", "Metrics", ...]
# Test patterns to skip for upstream tests
upstream_skip_patterns = ["AppWrapper", "PyTorch", "TrainJob", "Kueuectl", ...]
```

## Documentation

**Online Documentation:** https://trolocsis.com/kueue-dev/

The complete user guide is available as an mdBook:

```bash
# Install mdBook
cargo install mdbook

# Serve documentation locally
mdbook serve

# Build static HTML
mdbook build
```

Then open http://localhost:3000 to view the guide.

**Quick Links:**
- **[User Guide](docs/src/introduction.md)** - Complete mdBook documentation
- **[Implementation Plan](../KUEUE_DEV.md)** - Original implementation plan
- **[Phase 5 Complete](PHASE5_COMPLETE.md)** - Advanced features documentation
- **[Phase 6 Complete](PHASE6_COMPLETE.md)** - Polish and documentation details

## Development

### Project Structure

```
kueue-dev/
├── src/
│   ├── main.rs              # CLI entry point ✓
│   ├── lib.rs               # Library root ✓
│   ├── commands/            # Command implementations
│   │   ├── cleanup.rs       # Cleanup commands ✓
│   │   ├── cluster.rs       # Cluster commands ✓
│   │   ├── deploy.rs        # Deploy commands ✓
│   │   ├── test.rs          # Test commands ✓
│   │   ├── interactive.rs   # Interactive debugging menu ✓
│   │   └── openshift.rs     # OpenShift deployment ✓
│   ├── k8s/                 # Kubernetes operations
│   │   ├── images.rs        # Image loading ✓
│   │   ├── kind.rs          # Kind cluster management ✓
│   │   ├── kubectl.rs       # Kubectl wrapper ✓
│   │   └── nodes.rs         # Node operations ✓
│   ├── install/             # Component installation
│   │   ├── calico.rs        # Calico CNI ✓
│   │   ├── cert_manager.rs  # cert-manager ✓
│   │   ├── jobset.rs        # JobSet ✓
│   │   ├── leaderworkerset.rs # LeaderWorkerSet ✓
│   │   ├── operator.rs      # Operator deployment ✓
│   │   ├── prometheus.rs    # Prometheus operator ✓
│   │   └── olm.rs           # OLM installation ✓
│   ├── config/              # Configuration management
│   │   ├── images.rs        # Image config parser ✓
│   │   └── settings.rs      # TOML config support ✓
│   └── utils/               # Utilities ✓
│       ├── logger.rs        # Colored logging ✓
│       ├── prereqs.rs       # Prerequisite checking ✓
│       ├── container.rs     # Container runtime ✓
│       ├── prompt.rs        # User prompts ✓
│       ├── errors.rs        # Enhanced error handling ✓
│       ├── progress.rs      # Progress indicators ✓
│       ├── preflight.rs     # Preflight validation ✓
│       └── dryrun.rs        # Dry-run utilities ✓
├── docs/
│   └── USER_GUIDE.md        # Comprehensive user guide ✓
├── Cargo.toml
├── README.md
├── PHASE5_COMPLETE.md
└── PHASE6_COMPLETE.md
```

### Running Tests

```bash
cargo test
```

### Running with verbose output

```bash
kueue-dev -v <command>
```

## Implementation Status

- ✅ Phase 1: Foundation (COMPLETE)
  - ✅ Project structure and dependencies
  - ✅ Colored logging
  - ✅ Prerequisite checking
  - ✅ Container runtime detection
  - ✅ Basic CLI scaffolding

- ✅ Phase 2: Core Cluster Operations (COMPLETE)
  - ✅ Kind cluster creation/deletion/listing
  - ✅ Calico CNI installation
  - ✅ Worker node labeling
  - ✅ Kubeconfig management

- ✅ Phase 3: Component Installation (COMPLETE)
  - ✅ Image configuration parsing
  - ✅ cert-manager installation
  - ✅ JobSet and LeaderWorkerSet installation
  - ✅ CRD installation
  - ✅ Operator deployment with image handling
  - ✅ Image loading to kind clusters

- ✅ Phase 4: Test Integration (COMPLETE)
  - ✅ Ginkgo test execution
  - ✅ Test retry logic
  - ✅ Test skip pattern generation
  - ✅ Cleanup functionality

- ✅ Phase 5: Advanced Features (COMPLETE)
  - ✅ Prometheus operator installation with debugging
  - ✅ Interactive debugging menu
  - ✅ OLM bundle deployment support
  - ✅ OpenShift cluster deployment and testing

- ✅ Phase 6: Polish & Documentation (COMPLETE)
  - ✅ Enhanced error messages with actionable suggestions
  - ✅ Progress indicators for long operations
  - ✅ Shell completion scripts (bash, zsh, fish, powershell)
  - ✅ Multi-level logging (-v, -vv, -vvv)
  - ✅ Configuration file support (~/.config/kueue-dev/config.toml)
  - ✅ Preflight validation checks
  - ✅ Dry-run mode (--dry-run)
  - ✅ Comprehensive user guide and documentation

## License

Apache-2.0
