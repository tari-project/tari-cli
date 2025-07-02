---
Last Updated: 2025-06-26
Version: Latest (main branch)
Verified Against: crates/cli/src/project/config.rs:16-33
Test Sources: crates/cli/src/templates/collector.rs:154-200
Implementation: crates/cli/src/project/config.rs, crates/cli/src/cli/config.rs
---

# Configuration Reference

This document provides a complete reference for configuring Tari CLI projects.

## Project Configuration (`tari.config.toml`)

Every Tari project requires a `tari.config.toml` file in the project root for deployment and network settings.

### Network Configuration

<!-- SOURCE: crates/cli/src/project/config.rs:27-32 -->
<!-- VERIFIED: 2025-06-26 -->

```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

**Required Fields:**

- `wallet-daemon-jrpc-address` (string): JSON-RPC URL of the running Tari Wallet Daemon
    - **Default**: `http://127.0.0.1:9000/`
    - **Format**: Full HTTP/HTTPS URL with port
    - **Example**: `http://127.0.0.1:9000/` for local development

### CLI Configuration

The CLI itself can be configured via command-line arguments or environment variables.

## Template Configuration (`template.toml`)

Templates are discovered via `template.toml` descriptor files in template repositories.

### Basic Template Structure

<!-- SOURCE: crates/cli/src/templates/collector.rs:136-152 -->
<!-- VERIFIED: 2025-06-26 from test code -->

```toml
name = "basic-template"
description = "A basic Tari template for smart contract development"

# Optional: Custom template directory
[extra]
templates_dir = "templates"
```

**Required Fields:**

- `name` (string): Template identifier (converted to snake_case)
- `description` (string): Human-readable template description

**Optional Fields:**

- `[extra]` section: Additional template metadata
    - `templates_dir` (string): Subdirectory containing template files
    - `wasm_templates` (string): Directory for WASM-specific templates

### Template Repository Structure

Templates can be organized in two ways:

1. **Root-level templates**: `template.toml` in repository root
2. **Subdirectory templates**: `template.toml` in dedicated `templates/` folder

<!-- SOURCE: Test examples from collector.rs:158-172 -->
Example repository structure:

```
template-repo/
├── template.toml          # Root template
├── templates/             # Alternative: subdirectory templates
│   ├── basic/
│   │   └── template.toml
│   └── advanced/
│       └── template.toml
└── src/                   # Template source files
```

## Command-Line Arguments

### `tari create` (alias `new`) Options

<!-- SOURCE: crates/cli/src/cli/commands/create.rs:23-38 -->

```bash
tari create [OPTIONS] <NAME>

Arguments:
  <NAME>  Name of the project

Options:
  -t, --template <TEMPLATE>    Selected project template (ID)
      --output <PATH>          Output folder [default: current directory]
  -h, --help                   Print help
```

### `tari generate` (alias `gen`) Options

```bash
tari generate [OPTIONS] <NAME>

Arguments:
  <NAME>  Name of the template

Options:
  -t, --template <TEMPLATE>    Selected WASM template (ID)
      --output <PATH>          Output folder [default: current directory]
  -h, --help                   Print help
```

### `tari deploy` Options

<!-- SOURCE: crates/cli/src/cli/commands/deploy.rs:18-50 -->

```bash
tari deploy [OPTIONS] <TEMPLATE>

Arguments:
  <TEMPLATE>  Template project to deploy

Options:
  -a, --account <ACCOUNT>              Account for deployment fees
  -c, --custom-network <NETWORK>       Custom network name
  -y, --yes                            Confirm deployment without prompt
  -f, --max-fee <MAX_FEE>             Maximum deployment fee
      --project-folder <PATH>          Project folder [default: current directory]
  -h, --help                           Print help
```

## Environment Setup

### Prerequisites

1. **Tari Wallet Daemon**: Must be running and accessible
    - Download from: https://github.com/tari-project/tari-dan
    - Default address: `http://127.0.0.1:9000/`
    - Requires authentication with Admin permissions

2. **Rust Toolchain**: Required for template compilation
    - Install: `rustup target add wasm32-unknown-unknown`
    - WASM target is essential for smart contract compilation

3. **Project Structure**: Projects must contain:
    - `tari.config.toml`: Network configuration
    - `Cargo.toml`: Rust workspace configuration
    - Template directories with `template.toml` files

## Network Configurations

### Local Development

```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

### Custom Networks

For custom Tari networks, ensure the wallet daemon is configured for the target network:

```toml
[network]
wallet-daemon-jrpc-address = "http://custom-network-host:9000/"
```

Use the `--custom-network` flag when deploying:

```bash
tari deploy --account myaccount --custom-network testnet my_template
```

## Template Development

### Workspace Integration

When creating templates in existing Cargo workspaces, the CLI automatically:

- Detects workspace structure
- Updates `Cargo.toml` workspace members
- Maintains workspace dependencies and configuration

### WASM Compilation

Templates are compiled to WASM using:

```bash
cargo build --target wasm32-unknown-unknown --release
```

The CLI automatically handles this compilation during deployment.

## Troubleshooting

### Common Configuration Issues

1. **Wallet Daemon Connection Failed**
    - Verify `wallet-daemon-jrpc-address` in `tari.config.toml`
    - Ensure wallet daemon is running and accessible
    - Check network firewall settings

2. **Template Not Found**
    - Verify `template.toml` exists in template repository
    - Check template name matches exactly (case-sensitive)
    - Ensure git repository is accessible

3. **WASM Compilation Errors**
    - Install WASM target: `rustup target add wasm32-unknown-unknown`
    - Check Rust toolchain version compatibility
    - Verify template dependencies support WASM

4. **Deployment Insufficient Funds**
    - Check account balance in wallet daemon
    - Verify account name/address is correct
    - Consider using `--max-fee` to limit deployment costs

### Validation Commands

Test your configuration:

```bash
# Verify wallet daemon connection
curl -X POST http://127.0.0.1:9000/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"wallet.get_info", "params":{}}'

# Test template compilation
cd your_template_directory
cargo check --target wasm32-unknown-unknown
```
