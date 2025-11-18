# Deployment Failures

Troubleshoot operator deployment issues.

## Deployment Not Ready

**Error**: `Deployment failed to become ready`

### Diagnose

```bash
# Check pod status
kubectl get pods -n openshift-kueue-operator

# View pod logs
kubectl logs -n openshift-kueue-operator -l name=openshift-kueue-operator

# Check events
kubectl get events -n openshift-kueue-operator --sort-by='.lastTimestamp'
```

### Common Causes

1. **Image pull failures**
   - Verify images are loaded: `docker images | grep kueue`
   - Check images file: `kueue-dev images list --file <file>`

2. **CRD conflicts**
   - Clean existing resources: `kueue-dev cleanup`
   - Delete namespace: `kubectl delete ns openshift-kueue-operator`

3. **Resource constraints**
   - Check node resources: `kubectl top nodes`
   - Increase cluster size if needed

## cert-manager Issues

**Error**: cert-manager installation fails

### Solution

```bash
# Manual installation
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.3/cert-manager.yaml

# Verify
kubectl get pods -n cert-manager
```
