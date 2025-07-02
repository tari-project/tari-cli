---
title: Common Issues & Solutions
description: Solutions to frequently encountered problems when using Tari CLI
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: Real error messages from codebase and common usage patterns
audience: users
---

# Common Issues & Solutions

> **Quick solutions** to the most frequently encountered problems with Tari CLI

## 🚀 Installation Issues

### Cargo Install Failures

**Problem**: `cargo install tari-cli` fails with compilation errors

**Error Messages**:
```
error: failed to compile `tari-cli`
error[E0554]: `#![feature(...)]` may not be used on the stable release channel
```

**Solutions**:

1. **Update Rust Toolchain**:
   ```bash
   rustup update stable
   rustup default stable
   ```

2. **Install Required Targets**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Clear Cargo Cache**:
   ```bash
   cargo clean
   rm -rf ~/.cargo/registry/cache
   cargo install tari-cli --git https://github.com/tari-project/tari-cli --force
   ```

4. **Alternative: Use Pre-built Binaries**:
   ```bash
   # Download from releases instead
   curl -L https://github.com/tari-project/tari-cli/releases/latest/download/tari-cli-linux.tar.gz | tar xz
   ```

### Binary Permission Issues (Linux/macOS)

**Problem**: Downloaded binary cannot be executed

**Error Messages**:
```
permission denied: ./tari-cli
zsh: permission denied: tari-cli
```

**Solutions**:
```bash
# Fix permissions
chmod +x tari-cli

# Install to user directory
mkdir -p ~/.local/bin
mv tari-cli ~/.local/bin/
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify installation
tari --version
```

## 📋 Template Creation Issues

### Template Not Found Error

<!-- SOURCE: Verified against crates/cli/src/cli/commands/create.rs:42 -->
**Error Message**:
```
Template not found by name: basic. Possible values: ["advanced", "minimal", "nft"]
```

**Cause**: Template repositories not accessible or template name incorrect

**Solutions**:

1. **Check Internet Connection**:
   ```bash
   # Test git repository access
   git clone https://github.com/tari-project/wasm-template.git test-clone
   rm -rf test-clone
   ```

2. **Use Interactive Selection**:
   ```bash
   # Don't specify template, let CLI show options
   tari create my-project
   # Select from available templates
   ```

3. **Check Available Templates**:
   ```bash
   # Create project to see what's available
   tari create test-project
   # Note available templates, then cancel
   ```

4. **Clear Repository Cache**:
   ```bash
   # Remove cached repositories (if they exist)
   rm -rf ~/.local/share/tari_cli/template_repositories
   
   # Retry template creation
   tari create my-project
   ```

### Template Generation Fails

**Problem**: Template generation stops with cargo-generate errors

**Error Messages**:
```
Error: Failed to generate project from template
Error: template path does not exist
```

**Debugging Steps**:

1. **Verify Template Repository Structure**:
   ```bash
   # Clone repository manually to inspect
   git clone https://github.com/tari-project/wasm-template.git debug-templates
   cd debug-templates
   find . -name "template.toml" -exec echo "Found: {}" \;
   ```

2. **Check Template Descriptor Format**:
   <!-- SOURCE: Verified against test implementation in collector.rs -->
   ```toml
   # Ensure template.toml has required fields
   name = "template-name"
   description = "Template description"
   
   # Optional extra configuration
   [extra]
   templates_dir = "templates"
   ```

3. **Test with Minimal Template**:
   ```bash
   mkdir test-template && cd test-template
   cat > template.toml << EOF
   name = "test"
   description = "test template"
   EOF
   
   mkdir src
   echo 'fn main() { println!("Hello!"); }' > src/main.rs
   ```

## 🔧 Project Configuration Issues

### Wallet Daemon Connection Failed

**Error Messages**:
```
Connection refused (os error 61)
Failed to connect to wallet daemon at http://127.0.0.1:9000/
```

**Diagnostics**:
```bash
# Check if wallet daemon is running
curl -X POST http://127.0.0.1:9000/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"ping"}'

# Expected response: {"jsonrpc":"2.0","result":"pong","id":1}
```

**Solutions**:

1. **Start Wallet Daemon**:
   ```bash
   # Install and run Tari Wallet Daemon
   git clone https://github.com/tari-project/tari-dan.git
   cd tari-dan
   cargo build --release --bin tari_wallet_daemon
   ./target/release/tari_wallet_daemon --network localnet
   ```

2. **Verify Network Configuration**:
   <!-- SOURCE: Verified against crates/cli/src/project/config.rs:30 -->
   ```toml
   # In tari.config.toml
   [network]
   wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
   ```

3. **Test Different Port**:
   ```bash
   # Check if port is available
   netstat -an | grep 9000
   
   # Use alternative port if needed
   tari_wallet_daemon --json-rpc-address "127.0.0.1:9001"
   ```

4. **Check Firewall Settings**:
   ```bash
   # Linux: Check iptables
   sudo iptables -L
   
   # macOS: Check system firewall
   sudo pfctl -sr
   
   # Windows: Check Windows Firewall
   netsh advfirewall show allprofiles
   ```

### Authentication Errors

**Error Messages**:
```
Authentication failed
Insufficient permissions for admin operations
```

**Solutions**:

1. **Verify Admin Access**:
   ```bash
   # Ensure wallet daemon is configured for admin operations
   # Check wallet daemon startup logs for authentication setup
   ```

2. **Check Account Configuration**:
   ```bash
   # Verify account exists in wallet daemon
   # Account names are case-sensitive
   tari deploy --account "MyAccount" my-template
   ```

## 🔨 WASM Compilation Issues

### Missing WASM Target

**Error Messages**:
```
error[E0463]: can't find crate for 'std'
error: could not compile for target `wasm32-unknown-unknown`
```

**Solutions**:
```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Verify installation
rustup target list | grep wasm32-unknown-unknown
# Should show: wasm32-unknown-unknown (installed)

# Test compilation
cd templates/your-template
cargo check --target wasm32-unknown-unknown
```

### Compilation Failures

**Error Messages**:
```
error: linking with `rust-lld` failed: exit status: 1
error: could not compile `your-template`
```

**Debugging Steps**:

1. **Check Rust Version**:
   ```bash
   rustc --version
   # Ensure using stable Rust (1.70+)
   
   rustup default stable
   rustup update
   ```

2. **Verify Dependencies**:
   ```toml
   # In templates/your-template/Cargo.toml
   [dependencies]
   tari-template-lib = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
   serde = { version = "1.0", features = ["derive"] }
   
   # Avoid std-dependent crates for WASM
   # Remove: tokio, std::fs, reqwest, etc.
   ```

3. **Test Compilation Manually**:
   ```bash
   cd templates/your-template
   
   # Check syntax and dependencies
   cargo check --target wasm32-unknown-unknown
   
   # Full compilation
   cargo build --target wasm32-unknown-unknown --release
   
   # Verify WASM output
   ls target/wasm32-unknown-unknown/release/*.wasm
   ```

### WASM Binary Too Large

**Problem**: WASM binary exceeds size limits

**Solutions**:
```toml
# In Cargo.toml
[profile.release]
opt-level = "s"        # Optimize for size
lto = true            # Link-time optimization  
codegen-units = 1     # Single codegen unit
panic = "abort"       # Smaller panic handling
strip = true          # Remove debug symbols
```

**Additional Optimization**:
```bash
# Use wasm-opt for further optimization
cargo install wasm-pack
wasm-pack build --target web --out-dir pkg

# Or use wee_alloc for smaller memory footprint
# Add to Cargo.toml:
# wee_alloc = "0.4"
```

## 🚀 Deployment Issues

### Insufficient Funds

**Error Messages**:
```
Insufficient funds for deployment
Account balance: 1000 XTR, Required: 256875 XTR
```

**Solutions**:

1. **Check Account Balance**:
   ```bash
   # Use wallet daemon interface to check balance
   # Ensure account has sufficient XTR tokens
   ```

2. **Estimate Deployment Cost**:
   ```bash
   # See cost estimate before deploying
   tari deploy --account your-account your-template
   # Note the estimated cost in prompt
   ```

3. **Use Maximum Fee Limit**:
   ```bash
   # Limit deployment cost
   tari deploy --account your-account --max-fee 100000 your-template
   ```

4. **Request Test Tokens**:
   ```bash
   # For testnets, use faucet or request test tokens
   # For local development, create account with initial balance
   ```

### Network Connectivity Issues

**Error Messages**:
```
Network timeout
Failed to submit transaction to network
Connection refused to network endpoint
```

**Solutions**:

1. **Verify Network Configuration**:
   ```toml
   # Check tari.config.toml
   [network]
   wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
   ```

2. **Test Network Connection**:
   ```bash
   # Test wallet daemon connectivity
   telnet 127.0.0.1 9000
   
   # Test HTTP endpoint
   curl -v http://127.0.0.1:9000/
   ```

3. **Check Network Status**:
   ```bash
   # Verify local network is running
   ps aux | grep tari
   
   # Check network logs for issues
   tail -f ~/.tari/localnet/log/tari_wallet_daemon.log
   ```

### Template Validation Errors

**Error Messages**:
```
Template validation failed
Invalid WASM binary
Template does not meet network requirements
```

**Debugging**:

1. **Verify WASM Binary**:
   ```bash
   # Check WASM file exists and is valid
   ls -la target/wasm32-unknown-unknown/release/*.wasm
   
   # Verify file is not empty
   file target/wasm32-unknown-unknown/release/your_template.wasm
   ```

2. **Test WASM Validity**:
   ```bash
   # Install wasmtime for testing
   cargo install wasmtime-cli
   
   # Validate WASM binary
   wasmtime --invoke _start target/wasm32-unknown-unknown/release/your_template.wasm
   ```

3. **Check Template Interface**:
   ```rust
   // Ensure template follows required interface
   #[template]
   mod your_template {
       // Required: Component implementation
       // Required: Constructor function
       // Required: Proper error handling
   }
   ```

## 🔧 Workspace Integration Issues

### Cargo Workspace Conflicts

**Error Messages**:
```
Project is not a Cargo workspace project!
Failed to update workspace members
```

**Solutions**:

1. **Verify Workspace Structure**:
   ```toml
   # In root Cargo.toml
   [workspace]
   members = [
       "templates/*",  # Pattern to include all templates
   ]
   resolver = "2"
   ```

2. **Manual Workspace Update**:
   ```toml
   # If automatic update fails, manually add template
   [workspace]
   members = [
       "existing-template",
       "new-template-name"
   ]
   ```

3. **Create Outside Workspace**:
   ```bash
   # Generate template in separate directory if needed
   mkdir ../standalone-template
   cd ../standalone-template
   tari new my-template
   ```

## 🚀 Performance Issues

### Slow Template Discovery

**Problem**: CLI takes long time to discover templates

**Solutions**:

1. **Clear Repository Cache**:
   ```bash
   # Remove cached repositories
   rm -rf ~/.local/share/tari_cli/template_repositories
   ```

2. **Check Network Speed**:
   ```bash
   # Test git clone speed
   time git clone https://github.com/tari-project/wasm-template.git speed-test
   rm -rf speed-test
   ```

3. **Use Local Development**:
   ```bash
   # For development, consider local template repository
   git clone https://github.com/tari-project/wasm-template.git local-templates
   # Configure CLI to use local path (future feature)
   ```

### Slow Compilation

**Problem**: WASM compilation takes excessive time

**Solutions**:

1. **Optimize Build Configuration**:
   ```toml
   # In Cargo.toml
   [profile.dev]
   opt-level = 1    # Some optimization in debug builds
   
   [profile.release]
   codegen-units = 4  # Parallel compilation
   ```

2. **Use Build Cache**:
   ```bash
   # Install sccache for build caching
   cargo install sccache
   export RUSTC_WRAPPER=sccache
   
   # Verify cache is working
   sccache --show-stats
   ```

3. **Incremental Compilation**:
   ```bash
   # Enable incremental compilation
   export CARGO_INCREMENTAL=1
   
   # Use cargo-watch for development
   cargo install cargo-watch
   cargo watch -x "check --target wasm32-unknown-unknown"
   ```

## 🆘 Getting Help

### Debug Information Collection

When reporting issues, include this information:

```bash
# System information
echo "OS: $(uname -a)"
echo "Rust: $(rustc --version)"
echo "Cargo: $(cargo --version)"
echo "Tari CLI: $(tari --version)"

# Environment check
echo "WASM target: $(rustup target list | grep wasm32-unknown-unknown)"
echo "Wallet daemon: $(curl -s http://127.0.0.1:9000/ > /dev/null && echo "accessible" || echo "not accessible")"

# Test basic functionality
tari --help > /dev/null && echo "CLI functional: Yes" || echo "CLI functional: No"
```

### Enable Debug Logging

```bash
# Set environment variable for detailed logging
export RUST_LOG=debug
tari create debug-project

# Or for specific modules
export RUST_LOG=tari_cli::templates=debug
```

### Community Resources

- **🐛 Bug Reports**: [GitHub Issues](https://github.com/tari-project/tari-cli/issues)
- **💬 Community Discussion**: [Tari Discord](https://discord.gg/tari)
- **📚 Documentation**: [Complete guides](../README.md)
- **❓ Questions**: [GitHub Discussions](https://github.com/tari-project/tari-cli/discussions)

### Issue Reporting Template

When reporting issues, include:

1. **Exact command that failed**
2. **Complete error message**
3. **System information** (OS, Rust version, CLI version)
4. **Configuration files** (with sensitive info removed)
5. **Steps to reproduce**

---

**Still having issues?** Check our [Advanced Debugging Guide](debugging.md) or reach out to the community for help!
