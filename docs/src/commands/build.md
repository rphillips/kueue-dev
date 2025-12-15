# Build Commands

Build and push container images for kueue-operator components.

## Overview

The `kueue-dev images build` command builds and pushes container images for the kueue-operator project. It supports building operator, operand (kueue), must-gather, and bundle components.

## Usage

```bash
kueue-dev images build [OPTIONS] [COMPONENTS]...
```

### Arguments

- `[COMPONENTS]...` - Components to build (operator, operand, must-gather, bundle)
  - If not specified, builds all components

### Options

- `-i, --related-images <FILE>` - Path to images configuration file
  - If not specified, uses the config file setting
- `-p, --parallel` - Build components in parallel (faster for multiple components)
- `-v, --verbose` - Enable verbose output

## Valid Components

- `operator` - The kueue-operator image
- `operand` - The kueue (upstream) image
- `must-gather` - The must-gather debugging image
- `bundle` - The OLM bundle image

## Examples

### Build All Components

Build all four components (operator, operand, must-gather, bundle):

```bash
kueue-dev images build
```

This uses the images file from your config (`.kueue-dev.toml`) or the default `related_images.json`.

### Build Specific Components

Build only the operator:

```bash
kueue-dev images build operator
```

Build operator and operand:

```bash
kueue-dev images build operator,operand
```

### Use Custom Images File

Build with a custom images configuration file:

```bash
kueue-dev images build --related-images dev-images.json
```

Build specific components with custom images file:

```bash
kueue-dev images build operator --related-images my-images.json
```

### Build in Parallel

Build all components in parallel for faster builds:

```bash
kueue-dev images build --parallel
```

Build specific components in parallel:

```bash
kueue-dev images build operator,operand --parallel
```

Combine with custom images file:

```bash
kueue-dev images build --related-images dev-images.json --parallel
```

## Images Configuration File

The images file is a JSON file that specifies the image tags to build:

```json
[
  {
    "name": "operator",
    "image": "quay.io/myuser/kueue-operator:v0.1.0"
  },
  {
    "name": "operand",
    "image": "quay.io/myuser/kueue:v0.6.0"
  },
  {
    "name": "must-gather",
    "image": "quay.io/myuser/kueue-must-gather:v0.1.0"
  },
  {
    "name": "bundle",
    "image": "quay.io/myuser/kueue-bundle:v0.1.0"
  }
]
```

## Container Runtime

The build command automatically detects your container runtime:
- Tries `podman` first
- Falls back to `docker` if podman is not available

## Build Process

For each component, the command:

1. Validates the component name
2. Loads the image configuration
3. Detects the container runtime
4. Locates the appropriate Dockerfile:
   - **operator**: `Dockerfile` in project root
   - **operand**: `Dockerfile.kueue` in project root
   - **must-gather**: `must-gather/Dockerfile`
   - **bundle**: `bundle.developer.Dockerfile` in project root
5. Builds the image with the specified tag
6. Pushes the image to the registry

### Sequential vs Parallel Builds

**Sequential (default)**:
- Builds components one at a time
- Cleaner output with no interleaved logs
- Lower resource usage
- Simple status messages

**Parallel (`--parallel`)**:
- Builds all components simultaneously
- Significantly faster for multiple components
- Higher CPU and memory usage
- Live animated spinners for each component
- Color-coded status updates

### Build Output

The build command uses different output modes depending on whether parallel mode is enabled:

**Sequential Mode**:
```
Building and pushing container images...
==========================================
Building component: operator
==========================================
Image tag: quay.io/myuser/kueue-operator:latest
...
Successfully built and pushed: quay.io/myuser/kueue-operator:latest
```

**Parallel Mode**:
```
⠋ operator [3/4] Building image...
⠙ operand [2/4] Locating Dockerfile...
✓ must-gather Complete
```

Features:
- Animated spinners (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏)
- Color-coded components (blue/bold)
- Step indicators ([1/4], [2/4], etc.)
- Status symbols (✓ for success, ✗ for failure)
- Real-time updates
- Terminal title updates showing progress (e.g., "Building (2/3) - operand complete")

By default, stdout from Docker/Podman is suppressed. You'll only see:
- Spinner animations (in parallel mode)
- High-level status messages
- Error output (if builds fail)

To see full Docker/Podman output, enable debug logging:

```bash
# Show all build output
kueue-dev -vv images build

# Or set RUST_LOG
RUST_LOG=debug kueue-dev images build
```

## Configuration

Set a default images file in your `.kueue-dev.toml`:

```toml
[defaults]
images_file = "related_images.json"
```

This allows you to run `kueue-dev images build` without specifying the images file.

## Workflow Integration

### Local Development

```bash
# Make code changes
vim pkg/controllers/myfeature.go

# Build and push new images
kueue-dev images build --related-images dev-images.json

# Deploy to cluster
kueue-dev deploy operator kind --name dev --related-images dev-images.json
```

### CI/CD Pipeline

```bash
# Build all components in parallel (faster in CI)
kueue-dev images build --related-images ci-images.json --parallel

# Deploy and test
kueue-dev test operator --type kind --name ci --related-images ci-images.json
```

## Troubleshooting

### Build Fails

If the build fails, the error output from Docker/Podman will be displayed automatically. Common issues:

1. **Dockerfile exists**: Verify the Dockerfile is in the expected location
2. **Build context**: Ensure you're running from the project root
3. **Container runtime**: Verify podman or docker is installed and running
4. **Image permissions**: Ensure you're logged into the registry

For more detailed output during the build process, use debug logging:

```bash
kueue-dev -vv images build
```

### Push Fails

If push fails:

1. **Login to registry**:
   ```bash
   podman login quay.io
   # or
   docker login quay.io
   ```

2. **Check permissions**: Ensure you have push access to the repository

3. **Verify image tag**: Check the image name in your images JSON file

## Related

- [Deployment](./deploy.md) - Deploy built images
- [Images](./images.md) - Manage container images
- [Configuration](../configuration.md) - Configure default images file
