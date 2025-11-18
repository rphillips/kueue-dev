# Installation

This chapter covers installing kueue-dev and its prerequisites.

## Prerequisites

kueue-dev requires different tools depending on your use case:

### Required (All Scenarios)

- **kubectl** - Kubernetes CLI (>= v1.28.0)

### For Kind Clusters

- **kind** - Kubernetes in Docker (>= v0.20.0)
- **go** - Go programming language (>= 1.21)
- **Docker** or **Podman** - Container runtime

### For OpenShift

- **oc** - OpenShift CLI tool

### For OLM Deployment

- **operator-sdk** - Operator SDK for bundle deployment

## Checking Prerequisites

Use the built-in check command to verify your setup:

```bash
# Check basic prerequisites
kueue-dev check

# Check kind-specific prerequisites
kueue-dev check --kind

# Check OpenShift-specific prerequisites
kueue-dev check --openshift

# Check OLM prerequisites
kueue-dev check --olm

# Check everything
kueue-dev check --kind --openshift --olm
```

Example output:

```
Checking prerequisites...
Container runtime: Docker
  ✓ kubectl found (v1.30.0)
  ✓ kind found (v0.20.0)
  ✓ go found (1.21.0)
✓ All prerequisites satisfied!
```

## Building from Source

### Clone the Repository

```bash
cd kueue-operator/kueue-dev
```

### Build with Cargo

```bash
# Development build (faster)
cargo build

# Release build (optimized)
cargo build --release
```

The binary will be at:
- Development: `target/debug/kueue-dev`
- Release: `target/release/kueue-dev`

### Build with Nix

If you have Nix with flakes enabled:

```bash
# Enter development shell
nix develop .

# Build
cargo build --release
```

## Installing

### Install via Cargo

```bash
cargo install --path .
```

This installs kueue-dev to `~/.cargo/bin/kueue-dev`.

### Manual Installation

Copy the binary to a directory in your PATH:

```bash
# After building
cp target/release/kueue-dev ~/.local/bin/

# Or system-wide (requires sudo)
sudo cp target/release/kueue-dev /usr/local/bin/
```

## Verifying Installation

Check that kueue-dev is installed correctly:

```bash
# Check version
kueue-dev version

# Output:
# kueue-dev 0.1.0
# Development CLI tool for kueue-operator

# Verify help works
kueue-dev --help
```

## Shell Completion

Set up command completion for your shell:

### Bash

```bash
# Generate completion file
kueue-dev completion bash > /tmp/kueue-dev-completion.bash

# Install system-wide
sudo mv /tmp/kueue-dev-completion.bash /etc/bash_completion.d/kueue-dev

# Or per-user
mkdir -p ~/.local/share/bash-completion/completions
kueue-dev completion bash > ~/.local/share/bash-completion/completions/kueue-dev

# Reload shell
source ~/.bashrc
```

### Zsh

```bash
# Generate completion file
kueue-dev completion zsh > _kueue-dev

# Install to fpath location
sudo mv _kueue-dev /usr/local/share/zsh/site-functions/

# Or per-user
mkdir -p ~/.zsh/completions
kueue-dev completion zsh > ~/.zsh/completions/_kueue-dev

# Add to .zshrc if not already present
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload shell
source ~/.zshrc
```

### Fish

```bash
# Generate and install
kueue-dev completion fish > ~/.config/fish/completions/kueue-dev.fish

# Reload
source ~/.config/fish/config.fish
```

### PowerShell

```powershell
# Generate completion script
kueue-dev completion powershell > kueue-dev.ps1

# Add to profile
Add-Content $PROFILE ". $(Get-Location)\kueue-dev.ps1"
```

## Updating

To update kueue-dev to the latest version:

```bash
# Pull latest changes
cd kueue-operator/kueue-dev
git pull

# Rebuild and reinstall
cargo install --path . --force
```

## Uninstalling

```bash
# If installed via cargo
cargo uninstall kueue-dev

# If manually installed
rm ~/.local/bin/kueue-dev
# or
sudo rm /usr/local/bin/kueue-dev

# Remove completion scripts
sudo rm /etc/bash_completion.d/kueue-dev
sudo rm /usr/local/share/zsh/site-functions/_kueue-dev
rm ~/.config/fish/completions/kueue-dev.fish
```

## Next Steps

- [Configure kueue-dev](./configuration.md) with a config file
- [Quick Start](./quick-start.md) to deploy your first cluster
- [Command Reference](./commands/cluster.md) for detailed command documentation
