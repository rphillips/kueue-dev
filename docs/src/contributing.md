# Contributing

Help improve kueue-dev!

## Development Setup

### Prerequisites

- Rust (>= 1.70)
- cargo
- All kueue-dev prerequisites for testing

### Get Started

```bash
# Clone repository
git clone https://github.com/openshift/kueue-operator.git
cd kueue-operator/kueue-dev

# Enter Nix shell (if using Nix)
nix develop .

# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy

# Format code
cargo fmt
```

## Project Structure

```
kueue-dev/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── commands/            # Command implementations
│   ├── k8s/                 # Kubernetes operations
│   ├── install/             # Component installation
│   ├── config/              # Configuration
│   └── utils/               # Utilities
├── docs/                    # Documentation (mdBook)
├── Cargo.toml               # Dependencies
└── README.md                # Quick reference
```

## Guidelines

### Code Style

- Follow Rust conventions
- Run `cargo fmt` before committing
- Fix all `cargo clippy` warnings
- Add doc comments for public items

### Error Handling

Use the enhanced error types:

```rust
use crate::utils::KueueDevError;

return Err(KueueDevError::cluster_not_found("my-cluster")
    .suggest("Create cluster first")
    .into());
```

### Testing

- Add unit tests for new utilities
- Test commands manually on real clusters
- Update integration tests if needed

### Documentation

- Update relevant mdBook chapters
- Update README.md for new features
- Add examples to command help text
- Document breaking changes

## Making Changes

### 1. Create Branch

```bash
git checkout -b feature/my-feature
```

### 2. Make Changes

Follow the guidelines above.

### 3. Test

```bash
# Unit tests
cargo test

# Manual testing
cargo build
./target/debug/kueue-dev cluster create --name test
# ... test your changes ...
```

### 4. Commit

```bash
git add .
git commit -m "feat: add my feature"
```

Follow conventional commits format:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only
- `refactor:` - Code refactoring
- `test:` - Test changes

### 5. Push and Create PR

```bash
git push origin feature/my-feature
```

Open a pull request on GitHub.

## Areas for Contribution

### High Priority

- Additional command documentation
- More workflow examples
- Bug fixes
- Performance improvements

### Medium Priority

- Enhanced progress indicators
- Better error messages
- Additional preflight checks
- Unit test coverage

### Ideas Welcome

- New features
- Documentation improvements
- User experience enhancements
- Platform support (Windows, macOS)

## Questions?

- Open a GitHub Discussion
- Ask in pull request comments
- File an issue for bugs

Thank you for contributing! 🎉
