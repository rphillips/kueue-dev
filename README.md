# kueue-dev

Development CLI tool for kueue-operator.

**Documentation:** https://trolocsis.com/kueue-dev/

**Pre-built binaries:** [Releases](https://github.com/rphillips/kueue-dev/releases)

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Create a kind cluster
kueue-dev cluster create --name my-cluster --kubeconfig kube.kubeconfig

# Deploy operator to cluster
kueue-dev deploy kind --name my-cluster --related-images related_images.json

# Run tests
kueue-dev test operator
```

## Commands

### Cluster Management

```bash
kueue-dev cluster create --name my-cluster --kubeconfig kube.kubeconfig
kueue-dev cluster create --name my-cluster --cni default --kubeconfig kube.kubeconfig
kueue-dev cluster delete --name my-cluster [--force]
kueue-dev cluster list
```

### Deploy

```bash
kueue-dev deploy kind --name my-cluster --related-images related_images.json
kueue-dev deploy kind --name my-cluster --skip-tests --skip-kueue-cr
kueue-dev deploy kind --name my-cluster --kueue-frameworks BatchJob,Pod,JobSet
kueue-dev deploy kind --name my-cluster --cert-manager-version v1.17.0
kueue-dev deploy olm --bundle quay.io/my-org/bundle:latest --name my-cluster
kueue-dev deploy openshift --related-images related_images.json
```

Dependencies installed: cert-manager, JobSet, LeaderWorkerSet, Prometheus Operator, OLM (bundle mode).

### Testing

```bash
kueue-dev test run [--focus "pattern"] [--label-filter "filter"]
kueue-dev test operator [--type kind|openshift] [--kubeconfig path]
kueue-dev test upstream [--target singlecluster]
```

### Images

```bash
kueue-dev images build [--parallel]
kueue-dev images build operator,operand
kueue-dev images list --file related_images.json
kueue-dev images load --name my-cluster --related-images related_images.json
```

### Other Commands

```bash
kueue-dev check [--kind] [--openshift] [--olm]
kueue-dev cleanup [--kubeconfig path]
kueue-dev interactive [--kubeconfig path]
kueue-dev completion bash|zsh|fish|powershell
```

## Configuration

Create `~/.config/kueue-dev/config.toml` or `.kueue-dev.toml`:

```toml
[defaults]
cluster_name = "kueue-test"
cni_provider = "calico"
images_file = "related_images.json"
kubeconfig_path = "kube.kubeconfig"

[kueue]
namespace = "openshift-kueue-operator"
frameworks = ["BatchJob", "Pod", "Deployment", "StatefulSet", "JobSet", "LeaderWorkerSet"]

[versions]
cert_manager = "v1.18.0"
jobset = "v0.10.1"
leaderworkerset = "v0.7.0"
calico = "v3.28.2"
prometheus_operator = "v0.82.2"

[tests]
operator_skip_patterns = ["AppWrapper", "PyTorch", "Metrics"]
upstream_skip_patterns = ["AppWrapper", "PyTorch", "TrainJob", "Kueuectl"]
```

### Version Override Flags

```bash
--cert-manager-version v1.18.0
--jobset-version v0.10.1
--leaderworkerset-version v0.7.0
--prometheus-version v0.82.2
```

## License

Apache-2.0
