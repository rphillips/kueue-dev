# Introduction

Welcome to the **kueue-dev** user guide! This comprehensive documentation will help you master the kueue-dev CLI tool for developing, testing, and debugging both the kueue-operator and upstream Kueue.

## What is kueue-dev?

kueue-dev is a Rust-based CLI tool that provides a unified interface for:

- **Cluster Management** - Create and manage kind clusters with custom CNI configurations
- **Operator Deployment** - Deploy kueue-operator to kind, OpenShift, or via OLM bundles
- **Upstream Deployment** - Deploy upstream Kueue via kustomize or helm from source
- **Image Building** - Build and load container images for local development
- **Testing** - Run e2e tests with automatic retry and cleanup
- **Debugging** - Interactive menu for logs, metrics, and cluster inspection
- **Developer Tools** - Preflight checks, shell completions, and more

## Features at a Glance

- Kind cluster lifecycle management with CNI support (Calico, default)
- Operator deployment to kind and OpenShift (OLM bundle or direct manifests)
- Upstream Kueue deployment via kustomize or helm
- Image building from source with automatic kind loading
- Automatic dependency installation (cert-manager, JobSet, LeaderWorkerSet, Prometheus)
- E2e test execution with Ginkgo and automatic retry/cleanup
- Interactive debugging menu for logs, metrics, and cluster inspection
- Configuration file support (TOML) and shell completions
- Preflight validation and multi-level logging

## Quick Examples

### Deploy Operator to Kind

```bash
# Create cluster and deploy operator
kueue-dev cluster create --name dev
kueue-dev deploy operator kind --name dev

# Run tests
kueue-dev test run --focus "webhook"

# Cleanup
kueue-dev cluster delete --name dev
```

### Deploy Upstream Kueue from Source

```bash
# Create cluster
kueue-dev cluster create --name dev

# Deploy upstream kueue with locally built image
kueue-dev deploy upstream kustomize \
  --upstream-source /path/to/kueue/src \
  --build-image

# Or deploy with helm
kueue-dev deploy upstream helm \
  --upstream-source /path/to/kueue/src \
  --build-image
```

### Interactive Debugging

```bash
# Launch interactive menu
kueue-dev interactive

# View operator logs, metrics, cluster state, etc.
```

## Command Structure

```
kueue-dev
├── cluster          # Manage kind clusters (create, delete, list)
├── deploy
│   ├── operator     # Deploy kueue-operator
│   │   ├── kind     # Deploy to kind cluster
│   │   ├── olm      # Deploy via OLM bundle
│   │   └── openshift# Deploy to OpenShift
│   └── upstream     # Deploy upstream Kueue
│       ├── kustomize# Deploy using kustomize
│       └── helm     # Deploy using helm
├── test             # Run e2e tests
├── images           # Build and manage images
├── cleanup          # Clean up test resources
├── interactive      # Interactive debugging menu
├── check            # Verify prerequisites
├── completion       # Generate shell completions
└── version          # Show version information
```

## Getting Help

- Run `kueue-dev --help` for command-line help
- Run `kueue-dev <command> --help` for command-specific help
- Use `-vv` flag for detailed debug output
- Check the [FAQ](./faq.md) for common questions
- See [Troubleshooting](./troubleshooting/common-issues.md) for solutions

## Next Steps

- **New to kueue-dev?** Start with [Installation](./installation.md)
- **Ready to get started?** Jump to [Quick Start](./quick-start.md)
- **Deploy the operator?** See [Operator Deployment](./commands/deploy-operator.md)
- **Deploy upstream Kueue?** See [Upstream Deployment](./commands/deploy-upstream.md)
- **Need help?** See [Troubleshooting](./troubleshooting/common-issues.md)
