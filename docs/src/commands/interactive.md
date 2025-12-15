# Interactive Commands

Documentation for `kueue-dev interactive` commands.

## Overview

The interactive command launches a menu-driven interface for debugging and monitoring the kueue-operator deployment.

## Usage

```bash
kueue-dev interactive [OPTIONS]
```

**Options:**
- `-k, --kubeconfig <FILE>` - Path to kubeconfig file

## Menu Options

The interactive menu provides:

1. **Port-forward to Prometheus UI** - Access Prometheus at http://localhost:9090
2. **View Prometheus Operator logs** - Stream logs from the prometheus-operator pod
3. **View Prometheus instance logs** - Stream logs from the Prometheus pod
4. **View Kueue Operator logs** - Stream logs from the kueue-operator pod
5. **Show cluster information** - Display cluster status and resources
6. **Interactive kubectl shell** - Drop into a kubectl session

## Accessing Prometheus

### Via Interactive Menu

```bash
kueue-dev interactive
# Select "Port-forward to Prometheus UI"
# Open http://localhost:9090 in your browser
```

### Manual Port-Forward

```bash
# Port-forward to Prometheus service
kubectl port-forward -n default svc/prometheus-operated 9090:9090

# Open in browser
open http://localhost:9090
```

### Useful Prometheus Queries

Once in the Prometheus UI, try these queries:

```promql
# Kueue workload queue depth
kueue_pending_workloads

# Admission attempts
kueue_admission_attempts_total

# Workload state
kueue_workload_state

# Resource quota usage
kueue_cluster_queue_resource_usage
```

## Examples

```bash
# Launch interactive menu
kueue-dev interactive

# Launch with specific kubeconfig
kueue-dev interactive --kubeconfig /path/to/kubeconfig
```

## Related

- [Quick Start](../quick-start.md)
- [Common Workflows](../workflows.md)
- [Deploy Commands](./deploy.md)
