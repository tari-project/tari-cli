---
title: Publishing Guide
description: Complete guide to publishing Tari smart contracts to blockchain networks
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: crates/cli/src/cli/commands/publish.rs, crates/publish_lib/src/deployer.rs
audience: users
---

# Publishing Guide

> **✨ What you'll learn**: How to publish your Tari smart contracts to blockchain networks with confidence

## Overview

The Tari CLI publishing system handles the complete workflow from WASM compilation to blockchain publishing. It automatically builds your smart contract, estimates costs, verifies balances, and publishes to the network.

## Prerequisites

### Environment Setup

<!-- SOURCE: Verified against publish command implementation -->
Before publishing, ensure you have:

1. **Compiled Smart Contract**: Template must build successfully to WASM
2. **Tari Wallet Daemon**: Running and accessible at configured address
3. **Account with Funds**: Sufficient XTR balance for publishing fees
4. **Network Configuration**: Proper `tari.config.toml` setup

### Verify Prerequisites

```bash
# Check WASM compilation
cd templates/your-template
cargo build --target wasm32-unknown-unknown --release

# Verify wallet daemon connection
curl -X POST http://127.0.0.1:9000/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"ping"}'

# Expected response: {"jsonrpc":"2.0","result":"pong","id":1}
```

## Basic Publishing

### Simple Publishing

<!-- SOURCE: Actual CLI output from README.md and publish.rs -->
```bash
tari publish --account myaccount my_template
```

**Expected Output**:
```
✅ Init configuration and directories
✅ Refresh project templates repository
✅ Refresh wasm templates repository
✅ Building WASM template project "my_template"
❓ Publishing this template costs 256875 XTR (estimated), are you sure to continue? yes
✅ Publishing project "my_template" to local network
⭐ Your new template's address: f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774
```

### Auto-Confirmed Publishing

```bash
# Skip confirmation prompt
tari publish --account myaccount --yes my_template
```

### Publishing with Fee Limit

```bash
# Limit maximum publishing cost
tari publish --account myaccount --max-fee 100000 my_template
```

## Network Publishing

### Local Network Publishing

<!-- SOURCE: Verified against project/config.rs default configuration -->
Default configuration publishes to local development network:

```toml
# In tari.config.toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

Local publishing is ideal for:
- Development and testing
- Rapid iteration cycles
- Cost-free experimentation

### Custom Network Publishing

```bash
# Publish to testnet
tari publish --account testaccount --custom-network testnet my_template

# Publish to mainnet
tari publish --account mainaccount --custom-network mainnet my_template
```

**Network Configuration Requirements**:
1. **Wallet daemon** must be configured for target network
2. **Account** must exist on target network with sufficient balance
3. **Custom network name** must match your project configuration

## Publishing Process Deep Dive

### Step 1: Project Discovery

<!-- SOURCE: Verified against publish.rs implementation lines 57-79 -->
The CLI automatically locates your template within the Cargo workspace:

```bash
# CLI scans Cargo.toml workspace members
# Matches template name to package name (case-insensitive)
# Validates template exists in workspace
```

**Common Issues**:
- **Template not found**: Ensure package name matches argument
- **Workspace errors**: Run from project root with `Cargo.toml`

### Step 2: WASM Compilation

<!-- SOURCE: Verified against publish.rs build_project function lines 122-156 -->
Automatic WASM compilation with optimization:

```bash
# Equivalent command run automatically:
cargo build --target=wasm32-unknown-unknown --release
```

**Build Configuration**:
- **Target**: `wasm32-unknown-unknown` (required for Tari)
- **Mode**: Release build with optimizations
- **Output**: Binary at `target/wasm32-unknown-unknown/release/template_name.wasm`

### Step 3: Fee Estimation

<!-- SOURCE: Verified against deployer.rs publish_fee function lines 66-77 -->
The publishing system estimates costs before proceeding:

```rust
// Dry run to calculate fees
request.dry_run = true;
let response = client.publish_template(request).await?;
let estimated_fee = response.dry_run_fee;
```

**Fee Calculation**:
- Based on WASM binary size and complexity
- Network congestion factors
- Administrative overhead costs

### Step 4: Balance Verification

<!-- SOURCE: Verified against deployer.rs check_balance_to_deploy lines 92-120 -->
Balance check prevents failed publishing:

```rust
// Verify account has sufficient funds
let balances = client.accounts_get_balances(account).await?;
if balance < estimated_fee {
    return Err("Insufficient funds");
}
```

### Step 5: Template Publishing

<!-- SOURCE: Verified against deployer.rs deploy function lines 47-64 -->
Final publishing to blockchain:

```rust
// Submit template to Tari network
let template_address = client.publish_template(request).await?;
```

**Success Indicators**:
- Transaction accepted by network
- Template address returned
- Contract ready for interaction

## Advanced Publishing

### Batch Publishing

Publish multiple templates programmatically:

```bash
#!/bin/bash
ACCOUNT="publishing-account"

for template in templates/*/; do
    template_name=$(basename "$template")
    echo "Publishing $template_name..."
    tari publish --account "$ACCOUNT" --yes "$template_name"
done
```

### CI/CD Integration

GitHub Actions publishing example:

```yaml
name: Publish to Testnet
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
          
      - name: Install Tari CLI
        run: cargo install tari-cli --git https://github.com/tari-project/tari-cli
        
      - name: Publish contracts
        run: |
          tari publish --account ${{ secrets.TESTNET_ACCOUNT }} --yes my_contract
```

### Cross-Platform Publishing

<!-- SOURCE: Verified against Cross.toml and build_binaries.yml -->
The CLI supports publishing from multiple platforms:

- **Linux**: Native and cross-compiled (x86_64, arm64, riscv64)
- **macOS**: Intel and Apple Silicon (x86_64, arm64) 
- **Windows**: x64 and arm64 architectures

**Cross-compilation Setup**:
```bash
# Install cross-compilation tool
cargo install cross --git https://github.com/cross-rs/cross

# Deploy from any platform
cross build --target wasm32-unknown-unknown --release
```

## Network-Specific Considerations

### Local Development Network

**Advantages**:
- Free publishing (no real XTR cost)
- Fast confirmation times
- Complete control over network state
- Ideal for testing and iteration

**Setup**:
```bash
# Start local Tari wallet daemon
tari_wallet_daemon --network localnet
```

### Testnet Publishing

**Advantages**:
- Real network conditions
- Public accessibility
- Test token availability
- Production-like environment

**Configuration**:
```toml
# Custom testnet configuration
[network]
wallet-daemon-jrpc-address = "http://testnet-node:9000/"
```

### Mainnet Publishing

**Critical Considerations**:
- **Real XTR costs**: Publishing fees use actual cryptocurrency
- **Permanent publishing**: Cannot be undone or modified
- **Security critical**: Ensure code is thoroughly tested
- **Performance impact**: Consider gas optimization

**Best Practices**:
1. **Extensive Testing**: Publish to testnet first
2. **Code Audits**: Security review before mainnet
3. **Gas Optimization**: Minimize publishing costs
4. **Monitoring**: Track contract performance post-publishing

## Monitoring Published Templates

### Transaction Tracking

<!-- SOURCE: Based on deployer.rs transaction handling -->
Monitor your publishing transaction:

```bash
# Template address returned on successful publishing
# Use Tari block explorer to track:
# - Transaction confirmation
# - Contract initialization
# - Network propagation
```

### Contract Verification

Verify your published contract:

```bash
# Call contract methods to ensure proper publishing
# Check contract state initialization
# Validate expected functionality
```

## Troubleshooting Publishing

### Common Publishing Errors

**Insufficient Funds**:
```
Account balance: 1000 XTR, Required: 256875 XTR
```
**Solution**: Add funds to account or use `--max-fee` to limit cost

**Template Not Found**:
```
Project "my_template" not found!
```
**Solution**: Check template name matches Cargo.toml package name

**Compilation Failures**:
```
Failed to build project: /path/to/template
```
**Solution**: Fix Rust compilation errors, ensure WASM target installed

**Network Connection Issues**:
```
Connection refused (os error 61)
```
**Solution**: Verify wallet daemon is running and accessible

### Debug Mode

Enable detailed logging for troubleshooting:

```bash
# Enable debug output
RUST_LOG=debug tari publish --account myaccount my_template

# Focus on specific modules
RUST_LOG=tari_cli::commands::publish=debug tari publish --account myaccount my_template
```

### Validation Checklist

Before publishing, verify:

- [ ] **WASM compiles**: `cargo build --target wasm32-unknown-unknown --release`
- [ ] **Wallet connected**: `curl http://127.0.0.1:9000/` returns response
- [ ] **Account exists**: Account name is correct and accessible
- [ ] **Sufficient balance**: Account has more XTR than estimated fee
- [ ] **Network config**: `tari.config.toml` points to correct network
- [ ] **Template valid**: Package name matches publish argument

## Security Best Practices

### Pre-Publishing Security

1. **Code Review**: Audit smart contract logic thoroughly
2. **Test Coverage**: Comprehensive unit and integration tests
3. **Dependency Audit**: Check all crate dependencies for vulnerabilities
4. **Access Controls**: Verify contract permissions and ownership

### Publishing Security

1. **Account Security**: Use dedicated publishing accounts
2. **Network Verification**: Confirm publishing to intended network
3. **Fee Limits**: Use `--max-fee` to prevent cost overruns
4. **Confirmation**: Review all publishing details before confirming

### Post-Publishing Security

1. **Monitor Activity**: Track contract usage and transactions
2. **Update Procedures**: Plan for contract upgrades if supported
3. **Emergency Procedures**: Have response plan for security issues
4. **Access Management**: Secure administrative functions

## Cost Optimization

### Reducing Publishing Costs

1. **Optimize WASM Size**:
   ```toml
   # In Cargo.toml
   [profile.release]
   opt-level = "s"      # Optimize for size
   lto = true          # Link-time optimization
   codegen-units = 1   # Single codegen unit
   panic = "abort"     # Smaller panic handling
   strip = true        # Remove debug symbols
   ```

2. **Minimize Dependencies**: Use only essential crates for WASM target

3. **Code Efficiency**: Write gas-efficient smart contract logic

### Fee Estimation

Get publishing cost estimates:

```bash
# Dry run to see costs (publishing stops at confirmation)
tari publish --account myaccount my_template
# Note the estimated cost, then decline

# Set maximum fee based on estimate
tari publish --account myaccount --max-fee 300000 my_template
```

## Next Steps

### After Successful Publishing

1. **Record Template Address**: Save the returned template address
2. **Test Contract Functions**: Verify all methods work correctly
3. **Document Usage**: Update project documentation with publishing details
4. **Monitor Performance**: Track contract usage and costs

### Building Applications

**Frontend Integration**:
```javascript
// Example: Connect web application to published contract
const contractAddress = "f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774";
const contract = new TariContract(contractAddress);
```

**Backend Services**:
```rust
// Example: Server-side contract interaction
use tari_template_lib::prelude::*;

let result = contract.call_method("my_method", args).await?;
```

### Production Readiness

- **Load Testing**: Verify contract performance under load
- **Monitoring Setup**: Implement contract usage tracking
- **Documentation**: Create user guides and API documentation
- **Support**: Establish user support and bug reporting channels

---

**Ready for production publishing?** Review our [Security Best Practices](../04-troubleshooting/common-issues.md#publishing-issues) and [Monitoring Guide](../03-reference/cli-commands.md#publish-command) for enterprise-grade publishing.
