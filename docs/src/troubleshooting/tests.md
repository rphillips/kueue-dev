# Test Failures

Troubleshoot e2e test failures.

## Tests Failing

**Error**: Tests fail or timeout

### Diagnose

```bash
# Run with verbose output
kueue-dev -vv test run

# Focus on failing test
kueue-dev test run --focus "FailingTestName"

# Check operator logs
kueue-dev interactive
# Select option 4: View Kueue Operator logs
```

### Solutions

1. **Clean and retry**:
   ```bash
   kueue-dev cleanup
   kueue-dev test run
   ```

2. **Increase timeout**:
   Tests have automatic timeout handling, but operator may need time to reconcile.

3. **Check resource state**:
   ```bash
   kubectl get all -n openshift-kueue-operator
   kubectl get clusterqueues
   kubectl get resourceflavors
   ```

## Specific Test Issues

### Webhook Tests Fail

Ensure cert-manager is running:
```bash
kubectl get pods -n cert-manager
```

### Metrics Tests Fail

Verify Prometheus is configured (if using Prometheus).

### Network Policy Tests Fail

Ensure using Calico CNI:
```bash
kueue-dev cluster create --name test --cni calico
```
