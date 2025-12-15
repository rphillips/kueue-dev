# OLM Bundle Deployment

Deploy kueue-operator using Operator Lifecycle Manager bundles.

## Prerequisites

- `operator-sdk` installed
- Kind cluster or OpenShift cluster
- Bundle image pushed to registry

## Workflow

### 1. Create Cluster (if using kind)

```bash
kueue-dev cluster create --name olm-test
```

### 2. Deploy via OLM

```bash
kueue-dev deploy operator olm --bundle quay.io/my-org/kueue-bundle:v1.0.0 --name olm-test
```

This automatically:
- Installs OLM if not present
- Deploys the operator bundle
- Waits for operator to be ready

### 3. Verify Installation

```bash
kubectl get csv -n openshift-kueue-operator
kubectl get subscription -n openshift-kueue-operator
```

### 4. Run Tests

```bash
kueue-dev test run
```

## Notes

- OLM is installed automatically if needed
- Works with both kind and OpenShift clusters
- Bundle image must be publicly accessible or credentials configured
