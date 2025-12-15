# Frequently Asked Questions

Common questions about kueue-dev.

## General

### Q: What is kueue-dev?

A: kueue-dev is a Rust CLI tool that consolidates the development workflow for kueue-operator. It replaces multiple shell scripts with a single, well-tested tool.

### Q: Do I need Rust to use kueue-dev?

A: You need Rust to **build** kueue-dev from source, but once built, the binary runs standalone. Pre-built binaries may be available in releases.

### Q: Can I use kueue-dev in CI/CD?

A: Yes! kueue-dev is designed for both interactive and automated use. See [CI/CD Integration](./workflows/ci-cd.md).

## Usage

### Q: How do I use custom operator images?

A: Create a JSON file with your image references:

```json
{
  "operator": "quay.io/myuser/kueue-operator:my-tag",
  "operand": "quay.io/myuser/kueue:my-tag",
  "must-gather": "quay.io/myuser/kueue-must-gather:my-tag"
}
```

Then deploy: `kueue-dev deploy operator kind --related-images my-images.json`

### Q: Can I run multiple clusters simultaneously?

A: Yes! Use different cluster names:

```bash
kueue-dev cluster create --name cluster1
kueue-dev cluster create --name cluster2

kueue-dev deploy operator kind --name cluster1 --related-images images1.json
kueue-dev deploy operator kind --name cluster2 --related-images images2.json
```

### Q: How do I skip specific tests?

A: kueue-dev automatically skips disruptive tests. To run specific tests, use `--focus`:

```bash
kueue-dev test run --focus "webhook"
```

### Q: What CNI should I use?

A: **Calico** is recommended for most scenarios as it provides NetworkPolicy support tested by the operator. Use `default` CNI only for basic testing.

## Troubleshooting

### Q: How do I update the operator after code changes?

A:
1. Build new images (outside kueue-dev)
2. Clean resources: `kueue-dev cleanup`
3. Redeploy: `kueue-dev deploy operator kind --name <cluster> --related-images <file>`

### Q: Can I use kueue-dev with real Kubernetes clusters?

A: Yes! Use the `--kubeconfig` flag or `KUBECONFIG` environment variable. Be cautious as kueue-dev will make real changes.

### Q: How do I debug operator startup issues?

A: Use the interactive menu:

```bash
kueue-dev interactive
# Select option 4: View operator logs
# Select option 5: See deployment status
```

### Q: What files does kueue-dev modify?

A: kueue-dev only modifies Kubernetes cluster state. It creates:
- Kubeconfig files in `~/.kube/config-<cluster-name>`
- Temporary directories (automatically cleaned up)

Local files are never modified.

## Advanced

### Q: How do I contribute to kueue-dev?

A: See [Contributing](./contributing.md) for development guidelines.

### Q: Can I add custom commands?

A: Currently, kueue-dev doesn't support plugins, but you can fork and modify the source code. Feature requests are welcome!

### Q: How do I report bugs?

A: File an issue at: https://github.com/openshift/kueue-operator/issues

Include:
- kueue-dev version (`kueue-dev version`)
- Operating system
- Command that failed
- Output with `-vv` flag

## Configuration

### Q: Where should I put my config file?

A: Two options:
1. **Project-specific**: `.kueue-dev.toml` in project root
2. **Global**: `~/.config/kueue-dev/config.toml`

Project config takes precedence.

### Q: Can I disable colors?

A: Yes, in your config file:

```toml
[colors]
enabled = false
```

### Q: How do I set default cluster name?

A:

```toml
[defaults]
cluster_name = "my-default"
```

Then `kueue-dev cluster create` uses "my-default" automatically.

## More Questions?

- Check the [User Guide](./introduction.md)
- See [Troubleshooting](./troubleshooting/common-issues.md)
- Ask in GitHub Discussions
- File an issue for bugs
