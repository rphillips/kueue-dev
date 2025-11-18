# Cluster Connection Issues

Troubleshoot cluster connectivity problems.

## Cannot Connect to Cluster

**Error**: `Cannot connect to cluster` or `connection refused`

### Diagnose

```bash
# Check cluster is running
kind get clusters

# Test kubectl access
kubectl cluster-info

# Check kubeconfig
echo $KUBECONFIG
```

### Solutions

1. **Verify cluster exists**:
   ```bash
   kind get clusters
   kueue-dev cluster list
   ```

2. **Use correct kubeconfig**:
   ```bash
   export KUBECONFIG=~/.kube/config-<cluster-name>
   # or
   kueue-dev --kubeconfig ~/.kube/config-<cluster-name> test run
   ```

3. **Recreate cluster**:
   ```bash
   kueue-dev cluster delete --name <name>
   kueue-dev cluster create --name <name>
   ```

## Context Issues

**Error**: `The connection to the server was refused`

### Solution

```bash
# Switch to correct context
kubectl config use-context kind-<cluster-name>

# Or let kueue-dev handle it
kueue-dev deploy kind --name <cluster-name> --related-images <file>
```
