# OpenShift Deployment Workflow

Deploy and test kueue-operator on OpenShift clusters.

## Prerequisites

- OpenShift cluster access
- `oc` CLI tool installed
- Logged in with `oc login`

## Workflow

### 1. Verify Connection

```bash
kueue-dev check --openshift
```

### 2. Deploy to OpenShift

```bash
kueue-dev deploy openshift --related-images related_images.json
```

### 3. Run Tests

```bash
kueue-dev test operator --type openshift --focus "webhook"
```

### 4. Debug

```bash
kueue-dev interactive
```

The interactive menu works with OpenShift clusters using the current `oc` context.

## Notes

- Requires cluster-admin permissions for cert-manager installation
- Uses `oc` commands instead of `kubectl`
- Respects current OpenShift context
