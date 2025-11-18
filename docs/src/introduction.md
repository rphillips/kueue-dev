# Introduction

Welcome to the **kueue-dev** user guide! This comprehensive documentation will help you master the kueue-dev CLI tool for developing, testing, and debugging the kueue-operator.

## What is kueue-dev?

kueue-dev is a Rust-based CLI tool that provides a unified interface for:

- 🚀 **Cluster Management** - Create and manage kind clusters with custom CNI configurations
- 📦 **Deployment** - Deploy kueue-operator to kind, OpenShift, or via OLM bundles
- 🧪 **Testing** - Run e2e tests with automatic retry and cleanup
- 🔍 **Debugging** - Interactive menu for logs, metrics, and cluster inspection
- 🛠️ **Developer Tools** - Image management, preflight checks, and more

## Features at a Glance

- ✅ Cluster lifecycle management (create, delete, list)
- ✅ CNI support (Calico and default)
- ✅ Component installation (cert-manager, JobSet, LeaderWorkerSet)
- ✅ Operator deployment with custom images
- ✅ E2e test execution with Ginkgo
- ✅ Test retry logic and cleanup
- ✅ Prometheus operator with debugging
- ✅ Interactive debugging menu
- ✅ OLM bundle deployment
- ✅ OpenShift cluster support
- ✅ Enhanced error handling
- ✅ Progress indicators
- ✅ Shell completion scripts
- ✅ Multi-level logging
- ✅ Configuration file support
- ✅ Preflight validation
- ✅ Dry-run mode

## Quick Example

Here's a typical workflow:

```bash
# Create a cluster
kueue-dev cluster create --name dev --cni calico

# Build images (optional, if building from source)
kueue-dev images build --related-images dev-images.json

# Deploy operator
kueue-dev deploy kind --name dev --related-images dev-images.json

# Run tests
kueue-dev test run --focus "webhook"

# Debug interactively
kueue-dev interactive

# Cleanup
kueue-dev cleanup
kueue-dev cluster delete --name dev
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
- **Looking for specific commands?** Check the [Command Reference](./commands/cluster.md)
- **Need help?** See [Troubleshooting](./troubleshooting/common-issues.md)

Let's get started! 🚀
