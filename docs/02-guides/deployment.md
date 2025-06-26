---
title: Deployment Guide
description: Complete guide to deploying Tari smart contracts to blockchain networks
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: crates/cli/src/cli/commands/deploy.rs, crates/tari_deploy/src/deployer.rs
audience: users
---

# Deployment Guide

> **✨ What you'll learn**: How to deploy your Tari smart contracts to blockchain networks with confidence

## Overview

The Tari CLI deployment system handles the complete workflow from WASM compilation to blockchain deployment. It automatically builds your smart contract, estimates costs, verifies balances, and publishes to the network.

## Prerequisites

### Environment Setup

<!-- SOURCE: Verified against deployment command implementation -->
Before deploying, ensure you have:

1. **Compiled Smart Contract**: Template must build successfully to WASM
2. **Tari Wallet Daemon**: Running and accessible at configured address
3. **Account with Funds**: Sufficient XTR balance for deployment fees
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

## Basic Deployment

### Simple Deployment

<!-- SOURCE: Actual CLI output from README.md and deploy.rs -->
```bash
tari deploy --account myaccount my_template
```

**Expected Output**:
```
✅ Init configuration and directories
✅ Refresh project templates repository
✅ Refresh wasm templates repository
✅ Building WASM template project "my_template"
❓ Deploying this template costs 256875 XTR (estimated), are you sure to continue? yes
✅ Deploying project "my_template" to local network
⭐ Your new template's address: f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774
```

### Auto-Confirmed Deployment

```bash
# Skip confirmation prompt
tari deploy --account myaccount --yes my_template
```

### Deployment with Fee Limit

```bash
# Limit maximum deployment cost
tari deploy --account myaccount --max-fee 100000 my_template
```

## Network Deployment

### Local Network Deployment

<!-- SOURCE: Verified against project/config.rs default configuration -->
Default configuration deploys to local development network:

```toml
# In tari.config.toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

Local deployment is ideal for:
- Development and testing
- Rapid iteration cycles
- Cost-free experimentation

### Custom Network Deployment

```bash
# Deploy to testnet
tari deploy --account testaccount --custom-network testnet my_template

# Deploy to mainnet
tari deploy --account mainaccount --custom-network mainnet my_template
```

**Network Configuration Requirements**:
1. **Wallet daemon** must be configured for target network
2. **Account** must exist on target network with sufficient balance
3. **Custom network name** must match your project configuration

## Deployment Process Deep Dive

### Step 1: Project Discovery

<!-- SOURCE: Verified against deploy.rs implementation lines 57-79 -->
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

<!-- SOURCE: Verified against deploy.rs build_project function lines 122-156 -->
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
The deployment system estimates costs before proceeding:

```rust
// Dry run deployment to calculate fees
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
Balance check prevents failed deployments:

```rust
// Verify account has sufficient funds
let balances = client.accounts_get_balances(account).await?;
if balance < estimated_fee {
    return Err("Insufficient funds");
}
```

### Step 5: Template Deployment

<!-- SOURCE: Verified against deployer.rs deploy function lines 47-64 -->
Final deployment to blockchain:

```rust
// Submit template to Tari network
let template_address = client.publish_template(request).await?;
```

**Success Indicators**:
- Transaction accepted by network
- Template address returned
- Contract ready for interaction

## Advanced Deployment

### Batch Deployment

Deploy multiple templates programmatically:

```bash
#!/bin/bash
ACCOUNT="deployment-account"

for template in templates/*/; do
    template_name=$(basename "$template")
    echo "Deploying $template_name..."
    tari deploy --account "$ACCOUNT" --yes "$template_name"
done
```

### CI/CD Integration

GitHub Actions deployment example:

```yaml
name: Deploy to Testnet
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
        
      - name: Deploy contracts
        run: |
          tari deploy --account ${{ secrets.TESTNET_ACCOUNT }} --yes my_contract
```

### Cross-Platform Deployment

<!-- SOURCE: Verified against Cross.toml and build_binaries.yml -->
The CLI supports deployment from multiple platforms:

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
- Free deployments (no real XTR cost)
- Fast confirmation times
- Complete control over network state
- Ideal for testing and iteration

**Setup**:
```bash
# Start local Tari wallet daemon
tari_wallet_daemon --network localnet
```

### Testnet Deployment

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

### Mainnet Deployment

**Critical Considerations**:
- **Real XTR costs**: Deployment fees use actual cryptocurrency
- **Permanent deployment**: Cannot be undone or modified
- **Security critical**: Ensure code is thoroughly tested
- **Performance impact**: Consider gas optimization

**Best Practices**:
1. **Extensive Testing**: Deploy to testnet first
2. **Code Audits**: Security review before mainnet
3. **Gas Optimization**: Minimize deployment costs
4. **Monitoring**: Track contract performance post-deployment

## Monitoring Deployments

### Transaction Tracking

<!-- SOURCE: Based on deployer.rs transaction handling -->
Monitor your deployment transaction:

```bash
# Template address returned on successful deployment
# Use Tari block explorer to track:
# - Transaction confirmation
# - Contract initialization
# - Network propagation
```

### Contract Verification

Verify your deployed contract:

```bash
# Call contract methods to ensure proper deployment
# Check contract state initialization
# Validate expected functionality
```

## Troubleshooting Deployment

### Common Deployment Errors

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
RUST_LOG=debug tari deploy --account myaccount my_template

# Focus on specific modules
RUST_LOG=tari_cli::commands::deploy=debug tari deploy --account myaccount my_template
```

### Validation Checklist

Before deployment, verify:

- [ ] **WASM compiles**: `cargo build --target wasm32-unknown-unknown --release`
- [ ] **Wallet connected**: `curl http://127.0.0.1:9000/` returns response
- [ ] **Account exists**: Account name is correct and accessible
- [ ] **Sufficient balance**: Account has more XTR than estimated fee
- [ ] **Network config**: `tari.config.toml` points to correct network
- [ ] **Template valid**: Package name matches deployment argument

## Security Best Practices

### Pre-Deployment Security

1. **Code Review**: Audit smart contract logic thoroughly
2. **Test Coverage**: Comprehensive unit and integration tests
3. **Dependency Audit**: Check all crate dependencies for vulnerabilities
4. **Access Controls**: Verify contract permissions and ownership

### Deployment Security

1. **Account Security**: Use dedicated deployment accounts
2. **Network Verification**: Confirm deploying to intended network
3. **Fee Limits**: Use `--max-fee` to prevent cost overruns
4. **Confirmation**: Review all deployment details before confirming

### Post-Deployment Security

1. **Monitor Activity**: Track contract usage and transactions
2. **Update Procedures**: Plan for contract upgrades if supported
3. **Emergency Procedures**: Have response plan for security issues
4. **Access Management**: Secure administrative functions

## Cost Optimization

### Reducing Deployment Costs

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

Get deployment cost estimates:

```bash
# Dry run to see costs (deployment stops at confirmation)
tari deploy --account myaccount my_template
# Note the estimated cost, then decline

# Set maximum fee based on estimate
tari deploy --account myaccount --max-fee 300000 my_template
```

## Next Steps

### After Successful Deployment

1. **Record Template Address**: Save the returned template address
2. **Test Contract Functions**: Verify all methods work correctly
3. **Document Usage**: Update project documentation with deployment details
4. **Monitor Performance**: Track contract usage and costs

### Building Applications

**Frontend Integration**:
```javascript
// Example: Connect web application to deployed contract
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

**Ready for production deployment?** Review our [Security Best Practices](../04-troubleshooting/common-issues.md#deployment-issues) and [Monitoring Guide](../03-reference/cli-commands.md#deploy-command) for enterprise-grade deployments.
