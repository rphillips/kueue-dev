# CI/CD Integration

Integrate kueue-dev into your continuous integration pipeline.

## GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install kueue-dev
        run: cargo install --path kueue-dev
      - name: Build and test
        run: |
          # Build images in parallel for faster CI builds
          kueue-dev images build --related-images ci-images.json --parallel

          # Create cluster and deploy
          kueue-dev cluster create --name ci-test
          kueue-dev deploy operator kind --name ci-test --related-images ci-images.json

          # Run tests
          kueue-dev test run

          # Cleanup
          kueue-dev cleanup
          kueue-dev cluster delete --name ci-test
```

## Tips for CI

- Use `--dry-run` to validate commands first
- Disable colors and progress in CI: see [Configuration](../configuration.md)
- Use deterministic cluster names
- Always cleanup, even on failure

See example CI configurations in the repository.
