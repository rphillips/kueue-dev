# Utilities Commands

Documentation for `kueue-dev` utility commands.

## Overview

Utility commands provide system checks, completions, and version information.

## Commands

### check

Check all prerequisites for kueue-dev development.

```bash
kueue-dev check
```

This command checks for all required and optional tools needed for kueue-operator development:

**Checked tools:**
- `kubectl` - Kubernetes CLI (required)
- `kind` - Kubernetes in Docker (for local development)
- `go` - Go programming language (for building)
- `oc` - OpenShift CLI (for OpenShift deployments)
- `operator-sdk` - Operator SDK (for OLM deployments)
- Container runtime (`docker` or `podman`)

**Output:**

The command provides detailed output showing:
- Which tools are found (with ✓ indicator)
- Which tools are missing (with ✗ indicator and installation hints)
- Container runtime status
- Summary of found vs. missing tools

**Examples:**

```bash
# Check all prerequisites
kueue-dev check
```

**Sample output:**

```
[INFO] Checking all prerequisites...
[INFO]
[INFO] ✓ Container runtime: docker
[INFO]
[INFO] Found tools:
[INFO]   ✓ kubectl
[INFO]   ✓ kind
[INFO]   ✓ go
[INFO]   ✓ oc
[INFO]
[ERROR] Missing tools:
[ERROR]   ✗ operator-sdk - Install from https://sdk.operatorframework.io/
[INFO]
[INFO] ==========================================
[INFO] Summary:
[INFO]   Found: 4
[INFO]   Missing: 1
[INFO]   Container runtime: OK
[INFO] ==========================================
```

**Exit codes:**
- `0` - All prerequisites satisfied
- `1` - One or more prerequisites missing

**Note:** This command no longer requires flags like `--kind`, `--openshift`, or `--olm`. It now checks all tools automatically and provides a comprehensive report.

### completion

Generate shell completion scripts for kueue-dev.

```bash
kueue-dev completion <SHELL>
```

**Supported shells:**
- `bash`
- `zsh`
- `fish`
- `powershell`

**Examples:**

```bash
# Generate bash completion
kueue-dev completion bash > /etc/bash_completion.d/kueue-dev

# Generate zsh completion
kueue-dev completion zsh > "${fpath[1]}/_kueue-dev"

# Generate fish completion
kueue-dev completion fish > ~/.config/fish/completions/kueue-dev.fish
```

See [Completions](../advanced/completions.md) for detailed setup instructions.

## Related

- [Quick Start](../quick-start.md)
- [Completions](../advanced/completions.md)
- [Installation](../installation.md)
