# 🚀 Tari CLI

> **The complete toolkit for developing Tari smart contracts**

![GitHub Release](https://img.shields.io/github/v/release/tari-project/tari-cli)
![CI Build Status](https://img.shields.io/github/actions/workflow/status/tari-project/tari-cli/ci.yml)

The **Tari CLI** smart contract development tool for the Tari Layer-2 blockchain.

This CLI provides a streamlined workflow for building, testing, and deploying smart contracts on the Tari network. 
With a focus on simplicity and developer experience, the Tari CLI abstracts away complex blockchain interactions, 
allowing you to focus on writing your smart contract logic.

## ✨ What You Can Build

- **NFT Collections**: Create unique digital assets with custom metadata
- **Token Systems**: Build fungible tokens with advanced features  
- **DeFi Protocols**: Develop decentralized finance applications
- **Custom Templates**: Design reusable smart contract patterns

## 🚀 Quick Start

Get your first Tari smart contract deployed in under 5 minutes:

### 1. Install Tari CLI

```bash
# Using Cargo
cargo install tari-cli 

# Or download from releases
curl -L https://github.com/tari-project/tari-cli/releases/download/latest/tari-linux-x86_64.tar.gz | tar xz
```

### 2. Create Your First Project

<!-- SOURCE: Actual CLI output from README.md:49-57 -->
```bash
tari create my-first-contract

# ✅ Init configuration and directories
# ✅ Refresh project templates repository  
# ✅ Refresh wasm templates repository
# ✅ Collecting available project templates
# 🔎 Select project template: Basic - The basic project template to get started
# ✅ Generate new project
```

### 3. Add a Smart Contract

<!-- SOURCE: Actual CLI output from README.md:67-77 -->
```bash
cd my-first-contract
tari new MyToken

# ✅ Init configuration and directories
# ✅ Collecting available WASM templates  
# 🔎 Select WASM template: NFT - A simple NFT template to create your own
# ✅ Generate new project
# ✅ Update Cargo.toml
```

### 4. Deploy to Network

<!-- SOURCE: Actual CLI output from README.md:89-97 -->
```bash
tari deploy --account myaccount MyToken

# ✅ Building WASM template project "MyToken"
# ❓ Deploying this template costs 256875 XTR (estimated), are you sure to continue? yes
# ✅ Deploying project "MyToken" to local network  
# ⭐ Your new template's address: f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774
```

🎉 **Congratulations!** Your smart contract is live on Tari.

## 📚 Documentation

### 🎯 Essential Guides
- **[Getting Started](docs/01-getting-started/installation.md)** - Complete setup and first project
- **[Template Development](docs/02-guides/template-development.md)** - Creating custom smart contracts
- **[Configuration Guide](docs/02-guides/project-configuration.md)** - Project and network setup
- **[Deployment Guide](docs/02-guides/deployment.md)** - From build to blockchain

### 📖 Reference
- **[CLI Commands](docs/03-reference/cli-commands.md)** - Complete command reference
- **[Configuration Schema](docs/03-reference/configuration-schema.md)** - All configuration options
- **[API Patterns](docs/03-reference/api-patterns.md)** - Implementation patterns from real code

### 🔧 Help & Troubleshooting  
- **[Common Issues](docs/04-troubleshooting/common-issues.md)** - Solutions to frequent problems
- **[Advanced Debugging](docs/04-troubleshooting/debugging.md)** - Deep troubleshooting techniques
- **[FAQ](docs/04-troubleshooting/faq.md)** - Frequently asked questions

### 🤝 Contributing
- **[Development Setup](docs/05-contributing/development-setup.md)** - Contributor environment
- **[Testing Guide](docs/05-contributing/testing.md)** - Test framework and practices

## 🔧 Prerequisites

<!-- SOURCE: Verified against actual config defaults -->
Before using Tari CLI, ensure you have:

- **[Tari Wallet Daemon](https://github.com/tari-project/tari-dan)** running locally
- **Rust toolchain** with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

The CLI automatically detects your development environment and guides you through any missing setup.

## 🌐 Networks

- **Local Development**: Perfect for testing and iteration
- **Testnet**: Pre-production validation
- **Mainnet**: Production deployments

See the [Configuration Guide](docs/02-guides/project-configuration.md) for network-specific setup.

## 🆘 Get Help

- **📖 Documentation**: Comprehensive guides above
- **🐛 Bug Reports**: [GitHub Issues](https://github.com/tari-project/tari-cli/issues)
- **💬 Community**: [Tari Discord](https://discord.gg/tari)
- **📧 Questions**: [GitHub Discussions](https://github.com/tari-project/tari/discussions)

## 📊 Project Status

- **Build Status**: ![CI](https://img.shields.io/github/actions/workflow/status/tari-project/tari-cli/ci.yml)
- **Test Coverage**: ![Coverage](https://img.shields.io/codecov/c/github/tari-project/tari-cli)
- **Latest Release**: ![Release](https://img.shields.io/github/v/release/tari-project/tari-cli)

---

**Ready to build the future of decentralized applications?** [Get started now](docs/01-getting-started/installation.md) or explore our [template gallery](docs/02-guides/template-development.md#template-examples).
