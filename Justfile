# Justfile for kueue-dev

# Build the project
build:
    cargo build

# Build release version
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Check code without building
check:
    cargo check

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run all checks (format, lint, test)
ci: fmt-check lint test

# Install locally
install:
    cargo install --path .

# Clean build artifacts
clean:
    cargo clean

# Show help
help:
    @just --list
