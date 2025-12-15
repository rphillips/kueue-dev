# Local Development Workflow

The local development workflow is designed for iterative feature development and bug fixing on your local machine.

## Overview

This workflow allows you to:
- Quickly iterate on code changes
- Test features in isolation
- Debug issues interactively

## Step-by-Step Workflow

### 1. Create Development Cluster

```bash
kueue-dev cluster create --name dev --cni calico
```

### 2. Initial Deployment

```bash
kueue-dev deploy operator kind --name dev --related-images dev-images.json
```

### 3. Develop and Test Loop

```bash
# Make code changes
vim pkg/controllers/myfeature.go

# Build and push new images (use --parallel for speed)
kueue-dev images build --related-images dev-images.json --parallel

# Cleanup previous deployment
kueue-dev cleanup

# Redeploy with new images
kueue-dev deploy operator kind --name dev --related-images dev-images.json

# Test your changes
kueue-dev test run --focus "MyFeature"
```

### 4. Interactive Debugging

```bash
# Launch interactive menu
kueue-dev interactive

# Select option 4: View Kueue Operator logs
# Or option 5: Show cluster information
```

### 5. Cleanup When Done

```bash
kueue-dev cluster delete --name dev
```

## Tips

- Use a configuration file to avoid repeating flags
- Keep the same cluster across sessions
- Use `--skip-tests` during rapid iteration
- Enable verbose logging with `-v` or `-vv` for debugging

See [Configuration](../configuration.md) for setting up project-specific defaults.
