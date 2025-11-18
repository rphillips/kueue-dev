# Common Workflows

This chapter covers real-world development workflows using kueue-dev.

The following sections detail specific workflow patterns:

- **[Local Development](./workflows/local-development.md)** - Iterative development on your local machine
- **[CI/CD Integration](./workflows/ci-cd.md)** - Automated testing in continuous integration
- **[OpenShift Deployment](./workflows/openshift.md)** - Deploying to OpenShift clusters
- **[OLM Bundle Deployment](./workflows/olm.md)** - Using Operator Lifecycle Manager

## General Workflow Pattern

Most kueue-dev workflows follow this pattern:

```
1. Create/Select Environment
   ↓
2. Deploy Operator
   ↓
3. Test/Debug
   ↓
4. Iterate (modify code, redeploy)
   ↓
5. Cleanup
```

## Quick Workflow Examples

### Minimal Test Cycle

Test a single feature quickly:

```bash
kueue-dev cluster create --name quick-test
kueue-dev deploy kind --name quick-test --related-images dev-images.json
kueue-dev test run --focus "MyFeature"
kueue-dev cleanup
kueue-dev cluster delete --name quick-test
```

### Development with Iteration

Develop a feature with multiple test cycles:

```bash
# One-time setup
kueue-dev cluster create --name dev

# Iterate: code → deploy → test
while true; do
  # Make code changes...
  kueue-dev cleanup
  kueue-dev deploy kind --name dev --related-images dev-images.json
  kueue-dev test run --focus "MyFeature"
  read -p "Continue? (y/n) " -n 1 -r
  echo
  [[ ! $REPLY =~ ^[Yy]$ ]] && break
done

# Cleanup
kueue-dev cluster delete --name dev
```

### Full Test Suite

Run complete test suite before merging:

```bash
kueue-dev cluster create --name full-test --cni calico
kueue-dev deploy kind --name full-test --related-images related_images.json
kueue-dev test run  # All tests
kueue-dev cleanup
kueue-dev cluster delete --name full-test
```

Continue to the specific workflow guides for detailed examples.
