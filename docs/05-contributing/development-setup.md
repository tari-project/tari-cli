---
title: Development Setup Guide
description: Complete guide to setting up a development environment for contributing to Tari CLI
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: .github/workflows/ci.yml, Cargo.toml, Cross.toml
audience: contributors
---

# Development Setup Guide

> **Complete setup** for contributing to Tari CLI development

## Prerequisites

### System Requirements

<!-- SOURCE: Verified against CI configuration in .github/workflows/ci.yml -->
**Operating Systems**:
- **Linux**: Ubuntu 22.04+ (primary development platform)
- **macOS**: Intel and Apple Silicon supported
- **Windows**: Windows 10+ with WSL2 recommended

**Required Tools**:
- **Git**: Version control system
- **Rust**: Latest stable toolchain (1.84.0+ recommended)
- **Build tools**: Platform-specific development tools

### Rust Toolchain Setup

<!-- SOURCE: Verified against CI toolchain requirements -->
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.bashrc

# Verify installation
rustc --version
cargo --version

# Install required targets
rustup target add wasm32-unknown-unknown

# Install nightly for formatting
rustup toolchain install nightly-2025-01-09
rustup component add rustfmt --toolchain nightly-2025-01-09
```

### Platform-Specific Dependencies

#### Ubuntu/Debian

<!-- SOURCE: Verified against scripts/install_ubuntu_dependencies.sh -->
```bash
# Update package manager
sudo apt-get update

# Install development dependencies
sudo bash scripts/install_ubuntu_dependencies.sh

# Or install manually:
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    git \
    cmake \
    protobuf-compiler
```

#### macOS

```bash
# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install openssl cmake protobuf sqlite3

# For cross-compilation (optional)
brew install coreutils
```

#### Windows

```powershell
# Install chocolatey packages
choco install git
choco install cmake
choco install protoc

# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Use WSL2 for best development experience
wsl --install
```

## Repository Setup

### Clone Repository

```bash
# Clone the repository
git clone https://github.com/tari-project/tari-cli.git
cd tari-cli

# Create development branch
git checkout -b feature/my-feature
```

### Project Structure Overview

<!-- SOURCE: Verified against actual repository structure -->
```
tari-cli/
├── crates/
│   ├── cli/                    # Main CLI application
│   │   ├── src/
│   │   │   ├── cli/           # Command-line interface
│   │   │   ├── git/           # Git repository operations
│   │   │   ├── project/       # Project configuration
│   │   │   ├── templates/     # Template discovery and management
│   │   │   └── main.rs        # Application entry point
│   │   └── Cargo.toml         # CLI crate dependencies
│   └── tari_deploy/           # Deployment library
├── .github/
│   └── workflows/             # CI/CD pipeline definitions
├── scripts/                   # Build and utility scripts
├── docs/                      # Documentation
├── Cargo.toml                 # Workspace configuration
├── Cross.toml                 # Cross-compilation settings
└── rust-toolchain.toml       # Rust toolchain specification
```

### Development Dependencies

<!-- SOURCE: Verified against Cargo.toml workspace dependencies -->
**Core Dependencies**:
```toml
# From Cargo.toml
tari_wallet_daemon_client = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
tari_dan_engine = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
tari_template_lib = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
tokio = { version = "1.41.1", features = ["full"] }
clap = { version = "4.0", features = ["derive"] }
anyhow = "1.0"
```

## Build System

### Local Development Build

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Build specific crate
cargo build -p tari-cli

# Check code without building
cargo check
```

### Development Commands

<!-- SOURCE: Verified against CI pipeline commands -->
```bash
# Format code (uses nightly for consistency)
cargo +nightly-2025-01-09 fmt

# Run linting
cargo clippy --all-targets --all-features

# Install cargo-lints for enhanced checking
cargo install cargo-lints
cargo lints clippy --all-targets --all-features

# Check for unused dependencies
cargo install cargo-machete
cargo machete
```

### Testing Commands

```bash
# Install test runner
cargo install cargo-nextest --locked

# Run all tests
cargo nextest run --all-features

# Run tests with release build
cargo nextest run --all-features --release

# Run specific test
cargo nextest run --package tari-cli test_name

# Run with CI profile (from .config/nextest.toml)
cargo nextest run --all-features --release --profile ci
```

## Cross-Platform Development

### Cross-Compilation Setup

<!-- SOURCE: Verified against Cross.toml configuration -->
```bash
# Install cross-compilation tool
cargo install cross --git https://github.com/cross-rs/cross

# Build for different targets
cross build --target aarch64-unknown-linux-gnu
cross build --target x86_64-unknown-linux-gnu
cross build --target riscv64gc-unknown-linux-gnu
```

**Supported Targets** (from Cross.toml):
- `aarch64-unknown-linux-gnu` (ARM64 Linux)
- `x86_64-unknown-linux-gnu` (x86_64 Linux)
- `riscv64gc-unknown-linux-gnu` (RISC-V Linux)

### Environment Variables

<!-- SOURCE: Verified against Cross.toml passthrough configuration -->
```bash
# Common development environment variables
export RUST_LOG=debug
export RUST_BACKTRACE=1
export CARGO_INCREMENTAL=1

# Cross-compilation variables (automatically handled by Cross.toml)
export TARI_NETWORK=localnet
export TARI_TARGET_NETWORK=localnet
```

## Code Quality Standards

### Formatting Standards

<!-- SOURCE: Verified against CI fmt configuration -->
```bash
# Use nightly toolchain for consistent formatting
cargo +nightly-2025-01-09 fmt --all

# Check formatting without making changes
cargo +nightly-2025-01-09 fmt --all -- --check
```

### Linting Rules

```bash
# Standard clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Enhanced linting with cargo-lints
cargo lints clippy --all-targets --all-features
```

### Code Structure Guidelines

**Module Organization**:
- **Commands**: Each CLI command in separate module (`src/cli/commands/`)
- **Utilities**: Shared utilities in `src/cli/util.rs`
- **Configuration**: Config handling in `src/cli/config.rs`
- **Templates**: Template operations in `src/templates/`

**Error Handling**:
- Use `anyhow::Result` for application errors
- Use `thiserror` for domain-specific error types
- Provide helpful error messages with context

**Async Code**:
- Use `tokio` for async runtime
- Prefer `async/await` over manual futures
- Use structured concurrency patterns

## Testing Strategy

### Test Organization

<!-- SOURCE: Verified against actual test structure -->
```bash
# Unit tests (in each module)
cargo test --lib

# Integration tests
cargo test --test integration_tests

# Test with coverage (requires additional setup)
cargo install grcov
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage"
cargo test
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing -o coverage/
```

### Test Configuration

<!-- SOURCE: Verified against .config/nextest.toml -->
```toml
# .config/nextest.toml
[profile.ci]
slow-timeout = { period = "60s", terminate-after = 4 }

[profile.ci.junit]
path = "junit.xml"
```

### Writing Tests

**Template for new tests**:
```rust
#[tokio::test]
async fn test_feature_name() {
    // Setup
    let temp_dir = TempDir::new("test_feature").unwrap();
    
    // Execute
    let result = feature_function(temp_dir.path()).await;
    
    // Verify
    assert!(result.is_ok());
    // Add specific assertions
}
```

## Debugging Development

### Local CLI Testing

```bash
# Build and install locally
cargo install --path crates/cli --force

# Test CLI commands
tari --version
tari create test-project
```

### Debug Logging

```bash
# Enable detailed logging
export RUST_LOG=tari_cli=debug,tari_deploy=debug

# Test specific functionality
cargo run -- create test-project
cargo run -- new test-template
cargo run -- deploy --account test test-template
```

### Integration with Tari DAN

```bash
# Clone Tari DAN for wallet daemon
git clone https://github.com/tari-project/tari-dan.git ../tari-dan
cd ../tari-dan

# Build wallet daemon
cargo build --release --bin tari_wallet_daemon

# Run wallet daemon for testing
./target/release/tari_wallet_daemon --network localnet
```

## CI/CD Integration

### GitHub Actions Workflow

<!-- SOURCE: Verified against .github/workflows/ci.yml -->
The CI pipeline runs these checks:

1. **Formatting**: `cargo +nightly fmt --all -- --check`
2. **Linting**: `cargo lints clippy --all-targets --all-features`
3. **Building**: `cargo check --release --all-targets --locked`
4. **Testing**: `cargo nextest run --all-features --release --profile ci`
5. **License Check**: Validates file headers
6. **Dependency Audit**: Checks for unused dependencies

### Local CI Simulation

```bash
# Run the same checks as CI
./scripts/ci-local.sh

# Or run individual checks
cargo +nightly-2025-01-09 fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --release --all-targets --locked
cargo nextest run --all-features --release
```

## Contributing Workflow

### Development Process

1. **Create Feature Branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make Changes**:
   - Follow code quality standards
   - Add tests for new functionality
   - Update documentation as needed

3. **Test Locally**:
   ```bash
   cargo test
   cargo clippy
   cargo +nightly fmt
   ```

4. **Commit Changes**:
   ```bash
   git add .
   git commit -m "feat: add new feature description"
   ```

5. **Push and Create PR**:
   ```bash
   git push origin feature/my-feature
   # Create pull request on GitHub
   ```

### Commit Message Format

Follow conventional commits:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test additions/modifications
- `refactor:` Code refactoring
- `style:` Formatting changes
- `chore:` Maintenance tasks

### Pull Request Guidelines

**PR Template** (from `.github/PULL_REQUEST_TEMPLATE.md`):
- Clear description of changes
- Link to related issues
- Test coverage information
- Breaking change documentation
- Reviewer guidance

## Advanced Development

### Performance Profiling

```bash
# Profile CLI execution
cargo install flamegraph
cargo flamegraph --bin tari-cli -- create test-project

# Memory profiling with valgrind (Linux)
valgrind --tool=massif target/debug/tari-cli create test-project
```

### Custom Template Development

```bash
# Create local template repository for testing
mkdir -p /tmp/test-templates/basic-project
cat > /tmp/test-templates/basic-project/template.toml << EOF
name = "test-basic"
description = "Test project template"
EOF

# Test with custom repository
tari -e "project_template_repository.url=file:///tmp/test-templates" create test-project
```

### Dependency Management

```bash
# Update dependencies
cargo update

# Check for security vulnerabilities
cargo install cargo-audit
cargo audit

# Check dependency tree
cargo tree --duplicates
```

## Troubleshooting Development Issues

### Common Build Issues

**Linker errors**:
```bash
# Install mold linker for faster builds
sudo apt-get install mold  # Ubuntu
brew install mold          # macOS

# Use in development
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

**WASM target issues**:
```bash
# Reinstall WASM target
rustup target remove wasm32-unknown-unknown
rustup target add wasm32-unknown-unknown
```

**Dependency conflicts**:
```bash
# Clear cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache
cargo build
```

### Development Environment Reset

```bash
# Complete environment reset
cargo clean
rm -rf target/
rm -rf ~/.cargo/registry/cache
rustup update
cargo build
```

## Documentation Development

### Building Documentation

```bash
# Build Rust documentation
cargo doc --all --no-deps --open

# Serve documentation locally (requires mdbook)
cargo install mdbook
mdbook serve docs/
```

### Documentation Standards

- **Accuracy**: All examples must work with current codebase
- **Completeness**: Cover all public APIs and common use cases
- **Clarity**: Use clear, concise language
- **Examples**: Include working code examples
- **Updates**: Keep documentation synchronized with code changes

---

**Ready to contribute?** Check out [good first issues](https://github.com/tari-project/tari-cli/labels/good%20first%20issue) and our [Testing Guide](testing.md) for comprehensive testing practices.
