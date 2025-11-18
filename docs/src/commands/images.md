# Images Commands

Manage container images for kueue-operator.

## Overview

The `kueue-dev images` command provides tools for building, listing, and loading container images used by kueue-operator.

## Subcommands

### build

Build and push container images for kueue-operator components.

```bash
kueue-dev images build [OPTIONS] [COMPONENTS]...
```

**Arguments:**
- `[COMPONENTS]...` - Components to build (operator, operand, must-gather)
  - If not specified, builds all components

**Options:**
- `-i, --related-images <FILE>` - Path to images configuration file
- `-p, --parallel` - Build components in parallel with animated spinners

**Examples:**

```bash
# Build all components
kueue-dev images build

# Build specific components
kueue-dev images build operator,operand

# Build with custom images file
kueue-dev images build --related-images dev-images.json

# Build all components in parallel with live spinners
kueue-dev images build --parallel
```

**Parallel Mode Output:**
When using `--parallel`, each component gets its own animated spinner with real-time status:
```
⠋ operator [3/4] Building image...
⠙ operand [2/4] Locating Dockerfile...
✓ must-gather Complete
```

The terminal title bar also updates to show progress:
- `kueue-dev: Building 3 components` (initial)
- `kueue-dev: Building (2/3) - operand complete` (during)
- `kueue-dev: Build complete` (finished)

See [Build Commands](./build.md) for detailed documentation.

### list

List images from configuration file.

```bash
kueue-dev images list [OPTIONS]
```

**Options:**
- `-f, --file <FILE>` - Path to related images JSON file (default: `related_images.json`)

**Examples:**

```bash
# List images from default file
kueue-dev images list

# List images from custom file
kueue-dev images list --file my-images.json
```

**Output:**

```
Images from: related_images.json

  operator: quay.io/openshift/kueue-operator:latest
  operand: quay.io/openshift/kueue:latest
  must-gather: quay.io/openshift/kueue-must-gather:latest
```

### load

Load images from local container runtime to kind cluster.

```bash
kueue-dev images load [OPTIONS]
```

**Options:**
- `-n, --name <NAME>` - Cluster name (default: `kueue-test`)
- `--related-images <FILE>` - Path to related images JSON file (default: `related_images.json`)

**Examples:**

```bash
# Load images to default cluster
kueue-dev images load

# Load images to specific cluster
kueue-dev images load --name my-cluster

# Load images from custom file
kueue-dev images load --name dev --related-images dev-images.json
```

**Process:**

1. Reads image list from configuration file
2. Detects container runtime (podman or docker)
3. Loads each image into the kind cluster
4. Shows progress for each image

## Images Configuration File

All image commands use a JSON configuration file to specify image tags:

```json
{
  "operator": "quay.io/myuser/kueue-operator:v0.1.0",
  "operand": "quay.io/myuser/kueue:v0.6.0",
  "must-gather": "quay.io/myuser/kueue-must-gather:v0.1.0"
}
```

### Default Location

Set a default images file in your `.kueue-dev.toml`:

```toml
[defaults]
images_file = "related_images.json"
```

## Common Workflows

### Build, Load, and Deploy

```bash
# Build images in parallel
kueue-dev images build --related-images dev-images.json --parallel

# Load to kind cluster (if testing locally without pushing)
kueue-dev images load --name dev --related-images dev-images.json

# Deploy
kueue-dev deploy kind --name dev --related-images dev-images.json
```

### List Available Images

```bash
# Check what images will be used
kueue-dev images list --file related_images.json
```

### Quick Development Cycle

```bash
# 1. Make code changes
vim pkg/controllers/myfeature.go

# 2. Build new images (use --parallel for speed)
kueue-dev images build --parallel

# 3. Cleanup and redeploy
kueue-dev cleanup
kueue-dev deploy kind --name dev

# 4. Test
kueue-dev test run --focus "MyFeature"
```

## Container Runtime

All image commands automatically detect your container runtime:
- Tries `podman` first
- Falls back to `docker` if podman is not available

## Troubleshooting

### Images Not Found

If images cannot be loaded:

1. **Verify images exist locally**:
   ```bash
   podman images | grep kueue
   # or
   docker images | grep kueue
   ```

2. **Check configuration file**:
   ```bash
   kueue-dev images list
   ```

3. **Pull images if missing**:
   ```bash
   podman pull quay.io/openshift/kueue:latest
   # or
   docker pull quay.io/openshift/kueue:latest
   ```

### Load Fails

If loading to kind fails:

1. **Verify cluster exists**:
   ```bash
   kind get clusters
   ```

2. **Check kind CLI**:
   ```bash
   kind --version
   ```

3. **Verify runtime access**:
   ```bash
   podman ps
   # or
   docker ps
   ```

## Related

- [Build Commands](./build.md) - Detailed build documentation
- [Deployment](./deploy.md) - Deploy commands
- [Configuration](../configuration.md) - Configure default images file
