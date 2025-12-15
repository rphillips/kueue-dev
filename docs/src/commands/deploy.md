# Deploy Commands

Documentation for `kueue-dev deploy` commands.

## Overview

The `kueue-dev deploy` commands provide streamlined deployment workflows for both the kueue-operator and upstream Kueue.

```bash
kueue-dev deploy <SUBCOMMAND>
```

## Deployment Options

### Operator Deployment

Deploy the **kueue-operator** (OpenShift operator) which manages Kueue as an operand.

```bash
kueue-dev deploy operator <kind|olm|openshift>
```

Best for:
- OpenShift environments
- Production deployments
- Managed Kueue lifecycle

See [Operator Deployment](./deploy-operator.md) for full documentation.

**Quick Examples:**

```bash
# Deploy operator to kind cluster
kueue-dev deploy operator kind

# Deploy operator without OLM
kueue-dev deploy operator kind --no-bundle

# Deploy to OpenShift
kueue-dev deploy operator openshift
```

### Upstream Deployment

Deploy **upstream Kueue** directly from source using kustomize or helm.

```bash
kueue-dev deploy upstream <kustomize|helm>
```

Best for:
- Testing upstream Kueue changes
- Vanilla Kubernetes environments
- Development without the operator

See [Upstream Deployment](./deploy-upstream.md) for full documentation.

**Quick Examples:**

```bash
# Deploy with kustomize
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src

# Deploy with helm
kueue-dev deploy upstream helm --upstream-source /path/to/kueue/src

# Build image and deploy
kueue-dev deploy upstream kustomize --upstream-source /path/to/kueue/src --build-image
```

## Command Structure

```
kueue-dev deploy
├── operator              # Deploy kueue-operator
│   ├── kind              # Deploy to kind cluster
│   ├── olm               # Deploy via OLM bundle
│   └── openshift         # Deploy to OpenShift
└── upstream              # Deploy upstream Kueue
    ├── kustomize         # Deploy using kustomize
    └── helm              # Deploy using helm
```

## Comparison

| Feature | Operator Deployment | Upstream Deployment |
|---------|--------------------|--------------------|
| Target | kueue-operator | Upstream Kueue |
| Method | OLM bundle or manifests | Kustomize or Helm |
| Image source | Pre-built images | Pre-built or build from source |
| Use case | OpenShift, production | Development, testing |
| Kueue lifecycle | Managed by operator | Direct deployment |
| Dependencies | cert-manager, JobSet, LWS, Prometheus, OLM | cert-manager, JobSet, LWS |

## Common Options

Both deployment types share these common options:

| Option | Description |
|--------|-------------|
| `-c, --cluster-name` | Kind cluster name |
| `-k, --kubeconfig` | Path to kubeconfig file |
| `--skip-deps` | Skip dependency installation |
| `--cert-manager-version` | Override cert-manager version |
| `--jobset-version` | Override JobSet version |
| `--leaderworkerset-version` | Override LeaderWorkerSet version |

## Related

- [Operator Deployment](./deploy-operator.md)
- [Upstream Deployment](./deploy-upstream.md)
- [Quick Start](../quick-start.md)
- [Common Workflows](../workflows.md)
