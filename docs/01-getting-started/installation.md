---
title: Installation & Setup
description: Complete guide to installing Tari CLI and setting up your development environment
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: Installation methods and prerequisites from actual usage
audience: users
---

# Installation & Setup

> **✨ What you'll learn**: How to install Tari CLI and configure your development environment for smart contract
> development

## Installation Methods

### Using Cargo (Recommended)

<!-- SOURCE: README.md installation instructions -->

```bash
cargo install tari-cli --git https://github.com/tari-project/tari-cli --force
```

**Benefits**:

- Always installs the latest version
- Automatically handles Rust dependencies
- Works on all supported platforms

### Pre-built Binaries

Download from the [Releases page](https://github.com/tari-project/tari-cli/releases) for your platform:

**Linux (x86_64, arm64, riscv64)**:

```bash
# Download and extract
curl -L https://github.com/tari-project/tari-cli/releases/latest/download/tari-cli-linux.tar.gz | tar xz

# Make executable and install
chmod +x tari-cli
sudo mv tari-cli /usr/local/bin/
```

**macOS (x86_64, arm64)**:

```bash
# Download and extract  
curl -L https://github.com/tari-project/tari-cli/releases/latest/download/tari-cli-macos.tar.gz | tar xz

# Make executable and install
chmod +x tari-cli
sudo mv tari-cli /usr/local/bin/
```

**Windows (x64, arm64)**:

1. Download `tari-cli-windows.zip` from releases
2. Extract to a folder in your PATH
3. Run `tari-cli.exe` from Command Prompt or PowerShell

## Prerequisites

### 1. Rust Toolchain

Tari CLI requires Rust for compiling smart contracts to WebAssembly:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target (required for smart contracts)
rustup target add wasm32-unknown-unknown

# Verify installation
rustc --version
rustup target list | grep wasm32-unknown-unknown
```

### 2. Tari Wallet Daemon

<!-- SOURCE: README.md prerequisites section -->
The Tari Wallet Daemon handles authentication, account management, and network communication:

**Installation**:

```bash
# Clone the Tari DAN repository
git clone https://github.com/tari-project/tari-dan.git
cd tari-dan

# Build the wallet daemon
cargo build --release --bin tari_wallet_daemon

# Install (optional)
cargo install --path applications/tari_wallet_daemon
```

**Running**:

```bash
# Start for local development
tari_ootle_walletd --network localnet
# For testnet use the "igor" network:
tari_ootle_walletd --network igor

# A JSON-RPC server is started on http://127.0.0.1:9000/json_rpc by default and a web interface on http://127.0.0.1:5100/
```

**Verify Connection**:

```bash
curl -X POST http://127.0.0.1:9000/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"wallet.get_info", "params":{}}'
```

Expected response: `{"jsonrpc":"2.0","result":{"network":"igor","network_byte":36,"version":"0.10.4"},"id":1}`

## Verification

Verify your installation is working correctly:

### 1. Check CLI Installation

```bash
tari --version
# Should display version information

tari --help
# Should show available commands: create, new, deploy
```

### 2. Verify Development Environment

```bash
# Check Rust WASM target
rustup target list | grep wasm32-unknown-unknown
# Should show: wasm32-unknown-unknown (installed)

# Check wallet daemon (if running)
curl -s http://127.0.0.1:9000/ > /dev/null && echo "Wallet daemon accessible" || echo "Wallet daemon not running"
```

## Development Environment Setup

### Project Directory Structure

Create a dedicated workspace for your Tari projects:

```bash
mkdir ~/tari-projects
cd ~/tari-projects

# Your projects will be created here
# Each project contains configuration and templates
```

### IDE Configuration

**VS Code Extensions** (recommended):

- **rust-analyzer**: Rust language support
- **WebAssembly**: WASM file support
- **TOML Language Support**: Configuration files

**IDE Settings**:

```json
// .vscode/settings.json
{
  "rust-analyzer.cargo.features": [
    "all"
  ],
  "rust-analyzer.checkOnSave.allTargets": false,
  "rust-analyzer.cargo.allFeatures": false
}
```

## Network Configuration

### Local Development Network

<!-- SOURCE: Verified against crates/cli/src/project/config.rs:30 -->
The default configuration works for local development:

**Default wallet daemon address**: `http://127.0.0.1:9000/json_rpc`

This matches the Tari Wallet Daemon's default configuration and requires no additional setup.

### Custom Networks

For testnet or custom network deployments:

1. **Configure Wallet Daemon** for your target network
2. **Update project configuration** (covered in [Configuration Guide](../02-guides/project-configuration.md))
3. **Use deployment flags** for network-specific deployments

## Troubleshooting Installation

### Common Issues

**Cargo install fails**:

```bash
# Update Rust toolchain
rustup update stable

# Clear cargo cache
cargo clean

# Retry with verbose output
cargo install tari-cli --git https://github.com/tari-project/tari-cli --force --verbose
```

**WASM target missing**:

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Verify installation  
rustup target list | grep wasm32
```

**Wallet daemon connection issues**:

- Ensure daemon is running: `ps aux | grep tari_wallet_daemon`
- Check port availability: `netstat -an | grep 9000`
- Verify firewall settings allow local connections
- The wallet daemon will automatically use an OS-assigned port if 9000 is unavailable. Check the logs/stdout to see if
  this is the case.
- Consider configuring an unused port if 9000 is occupied

**Permission denied (Linux/macOS)**:

```bash
# Fix binary permissions
chmod +x tari-cli

# Install to user directory instead of system
mkdir -p ~/.local/bin
mv tari-cli ~/.local/bin/
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Getting Help

- **Installation Issues**: [GitHub Issues](https://github.com/tari-project/tari-cli/issues)
- **Environment Problems**: [Troubleshooting Guide](../04-troubleshooting/common-issues.md)
- **Build Failures**: [Development Setup](../05-contributing/development-setup.md)

## Next Steps

✅ **Installation Complete!**

**What's next?**

- 🚀 **[Quick Start Guide](quick-start.md)**: Create your first project in 5 minutes
- 📖 **[Development Workflow](workflow.md)**: Learn the complete development cycle
- 🔧 **[Configuration Guide](../02-guides/project-configuration.md)**: Customize your setup

---
**Need help?** Join the [Tari Discord](https://discord.gg/tari) or check our [FAQ](../04-troubleshooting/faq.md).
