---
title: Frequently Asked Questions
description: Common questions and answers about Tari CLI based on real user issues and code patterns
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: Common issues patterns from codebase and error handling
audience: users
---

# Frequently Asked Questions

> **Quick answers** to the most commonly asked questions about Tari CLI

## 🚀 Getting Started

### Q: What is Tari CLI?

**A**: Tari CLI is a command-line tool for developing smart contracts on the Tari Layer-2 blockchain. It helps you create projects, generate templates, and deploy smart contracts with a complete development workflow.

### Q: What can I build with Tari CLI?

**A**: You can build various types of smart contracts including:
- **NFT Collections**: Unique digital assets with custom metadata
- **Token Systems**: Fungible tokens with advanced features
- **DeFi Protocols**: Decentralized finance applications  
- **Custom Templates**: Reusable smart contract patterns

### Q: Do I need blockchain experience to use Tari CLI?

**A**: Basic programming knowledge is helpful, but Tari CLI is designed to be beginner-friendly. The template system provides working examples, and our [Quick Start Guide](../01-getting-started/quick-start.md) walks you through your first deployment.

## 🔧 Installation & Setup

### Q: How do I install Tari CLI?

**A**: Use one of these methods:

```bash
# Using Cargo (recommended)
cargo install tari-cli --git https://github.com/tari-project/tari-cli --force

# Or download pre-built binaries
curl -L https://github.com/tari-project/tari-cli/releases/latest/download/tari-cli-linux.tar.gz | tar xz
```

See our [Installation Guide](../01-getting-started/installation.md) for detailed instructions.

### Q: What are the system requirements?

**A**: You need:
- **Rust toolchain** with WASM target: `rustup target add wasm32-unknown-unknown`
- **Tari Wallet Daemon** running for deployments
- **Git** for template repository access
- **Internet connection** for downloading templates

### Q: Why do I get "wasm32-unknown-unknown target not found" errors?

<!-- SOURCE: Common compilation error pattern -->
**A**: Install the WASM target:

```bash
rustup target add wasm32-unknown-unknown

# Verify installation
rustup target list | grep wasm32-unknown-unknown
# Should show: wasm32-unknown-unknown (installed)
```

### Q: How do I set up the Tari Wallet Daemon?

**A**: Install and run the wallet daemon:

```bash
# Clone and build
git clone https://github.com/tari-project/tari-dan.git
cd tari-dan
cargo build --release --bin tari_wallet_daemon

# Run for local development
./target/release/tari_wallet_daemon --network localnet
```

## 📁 Projects & Templates

### Q: What's the difference between project templates and WASM templates?

**A**: 
- **Project templates**: Complete development environments with configuration and workspace setup
- **WASM templates**: Individual smart contract templates for specific use cases (NFT, tokens, etc.)

### Q: How do I create a new project?

**A**: Use the `create` command:

```bash
tari create my-project
# Follow interactive prompts to select a template
```

### Q: How do I add a smart contract to an existing project?

**A**: Use the `new` command from within your project:

```bash
cd my-project
tari new MyContract
# Select from available WASM templates
```

### Q: Can I use custom templates?

<!-- SOURCE: Verified against config.rs override functionality -->
**A**: Yes! Configure custom template repositories:

```bash
# Override template repository
tari -e "wasm_template_repository.url=https://github.com/my-org/templates" \
     new my-template

# Or configure permanently in ~/.config/tari_cli/tari.config.toml
```

### Q: Where are templates stored?

**A**: Templates are cached locally:
- **Location**: `~/.local/share/tari_cli/template_repositories/`
- **Auto-updated**: CLI refreshes templates before each use
- **Offline support**: Works with cached templates when offline

## 🔨 Development

### Q: How do I test my smart contract before deployment?

**A**: Test compilation locally:

```bash
cd templates/my-contract
cargo check --target wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

### Q: What's the basic structure of a Tari smart contract?

<!-- SOURCE: Verified against template examples in codebase -->
**A**: All Tari smart contracts follow this pattern:

```rust
use tari_template_lib::prelude::*;

#[template]
mod my_contract {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct MyContract {
        // Contract state
    }

    impl MyContract {
        pub fn new() -> Component<Self> {
            Component::new(Self {
                // Initialize state
            })
        }

        // Contract methods
    }
}
```

### Q: How do I add dependencies to my smart contract?

**A**: Edit the template's `Cargo.toml`:

```toml
[dependencies]
tari-template-lib = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
serde = { version = "1.0", features = ["derive"] }
# Add your dependencies here
```

### Q: Can I use external crates in my smart contract?

**A**: Yes, but they must be WASM-compatible. Avoid crates that use:
- Standard library networking (`std::net`)
- File system operations (`std::fs`)
- Threading (`std::thread`)
- Operating system APIs

## 🚀 Deployment

### Q: How much does deployment cost?

**A**: Deployment costs vary based on contract size and complexity. The CLI estimates costs before deployment:

```
❓ Deploying this template costs 256875 XTR (estimated), are you sure to continue?
```

### Q: How do I deploy to different networks?

**A**: Configure the network in your project's `tari.config.toml`:

```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"  # Local
# wallet-daemon-jrpc-address = "https://testnet:9000/"  # Testnet
```

Then deploy:
```bash
tari deploy --account myaccount my-contract
```

### Q: What happens after successful deployment?

**A**: You receive a template address that identifies your deployed contract:

```
⭐ Your new template's address: f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774
```

Use this address to interact with your contract from applications.

### Q: Can I update a deployed contract?

**A**: Smart contracts on Tari are immutable once deployed. To update functionality, you must deploy a new version with a new address.

## 🔧 Configuration

### Q: Where are configuration files located?

**A**: Tari CLI uses two configuration files:
- **Global**: `~/.config/tari_cli/tari.config.toml` (CLI settings)
- **Project**: `{project}/tari.config.toml` (network settings)

### Q: How do I change the default template repository?

<!-- SOURCE: Verified against config.rs default values -->
**A**: Edit your global config file:

```toml
# ~/.config/tari_cli/tari.config.toml
[wasm-template-repository]
url = "https://github.com/my-org/custom-templates"
branch = "main"
folder = "templates"
```

### Q: How do I override configuration for a single command?

**A**: Use the `-e` flag:

```bash
tari -e "wasm_template_repository.branch=development" \
     new my-template
```

## ❌ Error Resolution

### Q: "Template not found by name" error?

<!-- SOURCE: Verified against CreateHandlerError in create.rs -->
**A**: This means the template name doesn't match available options:

```bash
# Let CLI show available templates
tari create my-project
# Don't specify --template, select from the list

# Or clear template cache and retry
rm -rf ~/.local/share/tari_cli/template_repositories/
tari create my-project
```

### Q: "Connection refused" error during deployment?

**A**: The wallet daemon isn't running or accessible:

```bash
# Check if daemon is running
curl -X POST http://127.0.0.1:9000/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"ping"}'

# Start daemon if needed
tari_wallet_daemon --network localnet
```

### Q: "Insufficient funds" error?

**A**: Your account doesn't have enough XTR for deployment fees:
- For local development: Ensure your test account has sufficient balance
- For testnet: Request test tokens from faucet
- For mainnet: Add real XTR to your account

### Q: "Project is not a Cargo workspace" error?

**A**: The `new` command requires a Cargo workspace. Ensure your `Cargo.toml` contains:

```toml
[workspace]
members = [
    "templates/*",
]
resolver = "2"
```

### Q: Build errors with complex dependencies?

**A**: Ensure all dependencies support WASM compilation:

```bash
# Test WASM compatibility
cargo check --target wasm32-unknown-unknown

# Use WASM-specific alternatives
# Instead of: reqwest, tokio::fs, std::thread
# Use: web-compatible alternatives or async primitives
```

## 🔄 Workflows

### Q: What's the typical development workflow?

**A**: Follow this sequence:

1. **Create project**: `tari create my-project`
2. **Generate contract**: `tari new MyContract`  
3. **Develop logic**: Edit `templates/my_contract/src/lib.rs`
4. **Test compilation**: `cargo build --target wasm32-unknown-unknown --release`
5. **Deploy**: `tari deploy --account myaccount my_contract`

### Q: How do I manage multiple contracts in one project?

**A**: The workspace system handles multiple contracts automatically:

```bash
cd my-project
tari new ContractA  # Creates templates/contract_a/
tari new ContractB  # Creates templates/contract_b/

# Deploy individually
tari deploy --account myaccount contract_a
tari deploy --account myaccount contract_b
```

### Q: How do I integrate with CI/CD?

**A**: Use environment variables and automation:

```bash
# CI-friendly deployment
export TARI_ACCOUNT=ci-account
tari deploy --account $TARI_ACCOUNT --yes my-contract

# Batch deployment
for contract in templates/*/; do
    contract_name=$(basename "$contract")
    tari deploy --account $TARI_ACCOUNT --yes "$contract_name"
done
```

## 🌐 Networks & Environments

### Q: What networks does Tari CLI support?

**A**: Tari CLI works with any Tari network:
- **Local development**: Built-in support for testing
- **Testnet**: Public test network for validation
- **Mainnet**: Production network for live applications
- **Custom networks**: Configure any Tari-compatible network

### Q: How do I switch between networks?

**A**: Update your project's `tari.config.toml`:

```toml
[network]
# Development
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"

# Testnet  
# wallet-daemon-jrpc-address = "https://testnet-wallet:9000/"

# Custom
# wallet-daemon-jrpc-address = "https://my-network:9000/"
```

### Q: Can I deploy the same contract to multiple networks?

**A**: Yes! Each network deployment gets a unique address:

```bash
# Deploy to local
tari deploy --account local-account my-contract

# Deploy to testnet (after updating config)
tari deploy --account testnet-account my-contract
```

## 📊 Performance & Optimization

### Q: How do I optimize my smart contract for deployment costs?

**A**: Use these optimization techniques:

```toml
# In your template's Cargo.toml
[profile.release]
opt-level = "s"      # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit
panic = "abort"     # Smaller panic handling
strip = true        # Remove debug symbols
```

### Q: Why is template generation slow?

**A**: Common causes and solutions:
- **Network latency**: Templates download from GitHub
- **Large repositories**: Consider local template repositories
- **First run**: Initial cloning takes longer than updates

### Q: How do I speed up development iteration?

**A**: Use these techniques:
- **Keep wallet daemon running**: Avoid restart overhead
- **Use local templates**: Clone template repository locally
- **Cache dependencies**: Let Cargo cache build dependencies
- **Incremental builds**: Use `cargo check` for faster feedback

## 🔒 Security & Best Practices

### Q: Is it safe to use Tari CLI?

**A**: Tari CLI follows security best practices:
- **No private key handling**: Uses wallet daemon for security
- **Validated templates**: Official templates are reviewed
- **Open source**: Code is publicly auditable
- **Sandboxed execution**: WASM provides execution isolation

### Q: How do I audit my smart contract?

**A**: Follow these practices:
1. **Code review**: Review all contract logic thoroughly
2. **Test extensively**: Write comprehensive tests
3. **Dependency audit**: `cargo audit` to check for vulnerabilities
4. **Deploy to testnet first**: Validate in test environment
5. **Community review**: Share code for peer review

### Q: What should I never put in a smart contract?

**A**: Avoid these patterns:
- **Private keys or secrets**: Never hardcode sensitive data
- **Personal information**: Blockchain data is public
- **Large data structures**: Optimize for storage costs
- **External API calls**: Contracts can't make network requests
- **File system operations**: Not supported in WASM environment

---

## 🆘 Still Need Help?

If your question isn't answered here:

- **🐛 Bug Reports**: [GitHub Issues](https://github.com/tari-project/tari-cli/issues)
- **💬 Community Discussion**: [Tari Discord](https://discord.gg/tari)
- **📚 Documentation**: [Complete guides](../README.md)
- **❓ Questions**: [GitHub Discussions](https://github.com/tari-project/tari/discussions)
- **🔧 Advanced Issues**: [Debugging Guide](debugging.md)

**💡 Tip**: When asking for help, include:
- Exact command that failed
- Complete error message
- Operating system and Tari CLI version
- Steps to reproduce the issue
