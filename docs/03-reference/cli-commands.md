---
title: CLI Commands Reference
description: Complete reference for all Tari CLI commands, arguments, and options
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: crates/cli/src/cli/arguments.rs, command implementations
audience: users
---

# CLI Commands Reference

> **Complete reference** for all Tari CLI commands, arguments, and usage patterns

## Global Options

<!-- SOURCE: crates/cli/src/cli/arguments.rs:88-104 -->
Available for all commands:

```bash
tari [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

### Global Arguments

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--base-dir <PATH>` | `-b` | Base directory for CLI data | `~/.local/share/tari_cli` |
| `--config-file-path <PATH>` | `-c` | Config file location | `~/.config/tari_cli/tari.config.toml` |
| `--config-overrides <KEY=VALUE>` | `-e` | Config file overrides | None |

### Global Examples

```bash
# Use custom base directory
tari -b ~/my-tari-data create my-project

# Override configuration
tari -e "project_template_repository.url=https://github.com/my-org/templates" create my-project

# Use custom config file
tari -c ./custom-config.toml publish --account test my-template
```

## Commands Overview

<!-- SOURCE: crates/cli/src/cli/arguments.rs:121-138 -->
The Tari CLI provides three main commands:

| Command | Purpose | Typical Usage |
|---------|---------|---------------|
| **[`create`](#create-command)** | Creates new Tari template projects | Start new development workspace |
| **[`new`](#new-command)** | Creates new WASM template projects | Add smart contracts to existing projects |
| **[`publish`](#publish-command)** | Publishes templates to Tari network | Publish contracts to blockchain |

---

## `create` Command

<!-- SOURCE: crates/cli/src/cli/commands/create.rs:23-38 -->
Creates a new Tari templates project with complete development environment.

### Syntax

```bash
tari create [OPTIONS] <NAME>
```

### Arguments

| Argument | Type | Description | Validation |
|----------|------|-------------|------------|
| `<NAME>` | String | Name of the project | Converted to snake_case |

### Options

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `--template <TEMPLATE>` | `-t` | String | Selected project template ID | Interactive selection |
| `--target <PATH>` | | Path | Target folder for new project | Current directory |

### Behavior

1. **Repository Refresh**: Downloads/updates template repositories
2. **Template Discovery**: Scans for available project templates
3. **Template Selection**: Interactive selection or use specified template
4. **Project Generation**: Uses `cargo-generate` to create project structure
5. **Configuration**: Sets up `tari.config.toml` with network defaults

### Examples

**Basic Project Creation**:
```bash
# Interactive template selection
tari create my-defi-project

# Specify template directly
tari create my-nft-project --template basic

# Create in specific directory
tari create my-project --target ~/projects/tari/
```

**Expected Output**:
```
✅ Init configuration and directories
✅ Refresh project templates repository
✅ Refresh wasm templates repository
✅ Collecting available project templates
🔎 Select project template: Basic - The basic project template to get started
⠋ Generate new project[1/5] ⠁
✅ Generate new project
```

### Generated Structure

```
my-project/
├── Cargo.toml              # Workspace configuration
├── tari.config.toml        # Network settings
├── templates/              # Directory for WASM templates
└── README.md               # Project documentation
```

---

## `new` Command

<!-- SOURCE: crates/cli/src/cli/commands/new.rs:21-36 -->
Creates a new Tari WASM template (smart contract) within an existing project.

### Syntax

```bash
tari new [OPTIONS] <NAME>
```

### Arguments

| Argument | Type | Description | Validation |
|----------|------|-------------|------------|
| `<NAME>` | String | Name of the template | Converted to snake_case |

### Options

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `--template <TEMPLATE>` | `-t` | String | Selected WASM template ID | Interactive selection |
| `--target <PATH>` | | Path | Target folder for new template | Current directory |

### Behavior

1. **Repository Refresh**: Updates WASM template repositories
2. **Template Discovery**: Scans for available WASM templates
3. **Workspace Detection**: Automatically detects Cargo workspace
4. **Directory Selection**: Uses `templates/` subdirectory if available
5. **Template Generation**: Creates smart contract from template
6. **Workspace Update**: Adds new template to `Cargo.toml` workspace members

### Examples

**Basic Template Creation**:
```bash
# Interactive template selection
tari new MyNFT

# Specify template type
tari new MyToken --template fungible-token

# Create in specific directory
tari new MyDAO --target ./contracts/
```

**Expected Output**:
```
✅ Init configuration and directories
✅ Refresh project templates repository
✅ Refresh wasm templates repository
✅ Collecting available WASM templates
🔎 Select WASM template: NFT - A simple NFT template to create your own
⠋ Generate new project[1/10] ⠁
✅ Generate new project
✅ Update Cargo.toml
```

### Generated Structure

```
templates/my_nft/
├── Cargo.toml              # Template dependencies
├── src/
│   └── lib.rs             # Smart contract implementation
└── README.md              # Template documentation
```

---

## `publish` Command

<!-- SOURCE: crates/cli/src/cli/commands/publish.rs:18-50 -->
Publishes a Tari template to the blockchain network. (Also available as `deploy` for backward compatibility.)

### Syntax

```bash
tari publish [OPTIONS] <TEMPLATE>
```

### Arguments

| Argument | Type | Description | Validation |
|----------|------|-------------|------------|
| `<TEMPLATE>` | String | Template project to publish | Must exist in workspace |

### Options

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `--account <ACCOUNT>` | `-a` | String | Account for publishing fees | **Required** |
| `--custom-network <NETWORK>` | `-c` | String | Custom network name | Uses config default |
| `--yes` | `-y` | Flag | Confirm publishing without prompt | `false` |
| `--max-fee <MAX_FEE>` | `-f` | u64 | Maximum publishing fee | Auto-estimated |
| `--project-folder <PATH>` | | Path | Project folder location | Current directory |

### Behavior

1. **Configuration Loading**: Reads `tari.config.toml` for network settings
2. **Project Discovery**: Locates template in Cargo workspace
3. **WASM Compilation**: Builds template for `wasm32-unknown-unknown` target
4. **Fee Estimation**: Calculates publishing cost
5. **Balance Verification**: Checks account has sufficient funds
6. **Publishing**: Submits template to Tari network via wallet daemon
7. **Address Return**: Provides published template address

### Examples

**Basic Publishing**:
```bash
# Publish with confirmation prompt
tari publish --account myaccount my_nft

# Publish with auto-confirmation
tari publish --account myaccount --yes my_token

# Publish to custom network
tari publish --account testaccount --custom-network testnet my_dao

# Publish with fee limit
tari publish --account myaccount --max-fee 100000 my_template
```

**Expected Output**:
```
✅ Init configuration and directories
✅ Refresh project templates repository  
✅ Refresh wasm templates repository
✅ Building WASM template project "my_nft"
❓Publishing this template costs 256875 XTR (estimated), are you sure to continue? yes
✅ Publishing project "my_nft" to local network
⭐ Your new template's address: f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774
```

### Prerequisites

- **Wallet Daemon**: Must be running and accessible
- **Account**: Must exist with sufficient balance
- **Network Access**: Must be configured for target network
- **WASM Target**: `wasm32-unknown-unknown` must be installed

---

## Error Handling

### Common Error Messages

<!-- SOURCE: crates/cli/src/cli/commands/create.rs:42 -->
**Template Not Found**:
```
Template not found by name: basic. Possible values: ["advanced", "minimal", "nft"]
```
**Solution**: Use `tari create` without `--template` to see available options.

**Workspace Not Found**:
```
Project is not a Cargo workspace project!
```
**Solution**: Run command from project root with `Cargo.toml` workspace.

**Account Not Found**:
```
Account "nonexistent" not found
```
**Solution**: Verify account exists in wallet daemon.

**Insufficient Funds**:
```
Insufficient funds for publishing
```
**Solution**: Add funds to account or use `--max-fee` to limit cost.

### Debugging Commands

**Check CLI Configuration**:
```bash
# Verify CLI installation
tari --version

# Show help for specific command
tari create --help
tari new --help  
tari publish --help
```

**Test Repository Access**:
```bash
# This will test template repository connectivity
tari create test-connectivity

# Cancel when template selection appears
# Success = repositories are accessible
```

**Verify Wallet Connection**:
```bash
curl -X POST http://127.0.0.1:9000/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"ping"}'
```

## Environment Variables

While not directly supported, these environment variables affect CLI behavior:

| Variable | Effect | Example |
|----------|--------|---------|
| `RUST_LOG` | Enable debug logging | `RUST_LOG=debug tari create my-project` |
| `CARGO_TARGET_DIR` | Override cargo build directory | `CARGO_TARGET_DIR=./build tari publish` |
| `HOME` | Affects default directories | Automatically detected |

## Configuration Integration

Commands interact with configuration files:

- **Global CLI Config**: `~/.config/tari_cli/tari.config.toml`
- **Project Config**: `project_dir/tari.config.toml`
- **Command-line Overrides**: `--config-overrides KEY=VALUE`

**Precedence**: CLI args > Project config > Global config > Defaults

## Advanced Usage

### Scripting and Automation

**Automated Project Creation**:
```bash
#!/bin/bash
PROJECT_NAME="automated-project"
TEMPLATE_TYPE="basic"

tari create "$PROJECT_NAME" --template "$TEMPLATE_TYPE"
cd "$PROJECT_NAME"
tari new "MyContract" --template "nft"
```

**Batch Publishing**:
```bash
#!/bin/bash
ACCOUNT="publishing-account"

for template in templates/*/; do
    template_name=$(basename "$template")
    tari publish --account "$ACCOUNT" --yes "$template_name"
done
```

### Integration with Build Systems

**Makefile Integration**:
```makefile
publish: build
	tari publish --account $(ACCOUNT) --yes $(TEMPLATE)

build:
	cargo build --target wasm32-unknown-unknown --release

test:
	cargo test --workspace
```

---

For complete examples and advanced usage patterns, see:
- **[Quick Start Guide](../01-getting-started/quick-start.md)** - End-to-end examples
- **[Template Development](../02-guides/template-development.md)** - Custom template creation
- **[Troubleshooting](../04-troubleshooting/common-issues.md)** - Issue resolution
