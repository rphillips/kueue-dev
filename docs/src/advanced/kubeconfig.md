# Custom Kubeconfig

Use specific kubeconfig files with kueue-dev.

## Methods

### 1. Flag

```bash
kueue-dev test run --kubeconfig /path/to/kubeconfig
```

### 2. Environment Variable

```bash
export KUBECONFIG=/path/to/kubeconfig
kueue-dev test run
```

### 3. Kind Cluster Auto-Config

When using kind clusters, kueue-dev automatically creates kubeconfig at:

```
~/.kube/config-<cluster-name>
```

## Multiple Clusters

Switch between clusters easily:

```bash
# Cluster 1
kueue-dev deploy kind --name cluster1 --related-images images1.json

# Cluster 2  
kueue-dev deploy kind --name cluster2 --related-images images2.json

# Test on cluster 1
kueue-dev test run --kubeconfig ~/.kube/config-cluster1

# Test on cluster 2
kueue-dev test run --kubeconfig ~/.kube/config-cluster2
```
