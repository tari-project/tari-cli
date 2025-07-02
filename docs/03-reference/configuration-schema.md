---
title: Configuration Schema Reference
description: Complete reference for all Tari CLI configuration options and file formats
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: crates/cli/src/cli/config.rs, crates/cli/src/project/config.rs
audience: users
---

# Configuration Schema Reference

> **Complete reference** for all Tari CLI configuration files, options, and environment variables

## Configuration Hierarchy

The Tari CLI uses a hierarchical configuration system with the following precedence (highest to lowest):

1. **Command-line arguments** (`--account`, `--max-fee`, etc.)
2. **CLI overrides** (`-e KEY=VALUE`)
3. **Project configuration** (`project_dir/tari.config.toml`)
4. **Global CLI configuration** (`~/.config/tari_cli/tari.config.toml`)
5. **Built-in defaults**

## Global CLI Configuration

<!-- SOURCE: Verified against crates/cli/src/cli/config.rs -->
### File Location

**Default path**: `~/.config/tari_cli/tari.config.toml`

**Custom path**: Use `--config-file-path` or `-c` flag

### CLI Configuration Schema

```toml
# ~/.config/tari_cli/tari.config.toml

[project-template-repository]
url = "https://github.com/tari-project/wasm-template"
branch = "main"
folder = "project_templates"

[wasm-template-repository]  
url = "https://github.com/tari-project/wasm-template"
branch = "main"
folder = "wasm_templates"
```

### CLI Configuration Fields

<!-- SOURCE: Verified against config.rs lines 22-51 -->
#### `[project-template-repository]`

Controls where project templates are sourced from:

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `url` | String | Git repository URL for project templates | `"https://github.com/tari-project/wasm-template"` |
| `branch` | String | Git branch to use | `"main"` |
| `folder` | String | Subdirectory containing templates | `"project_templates"` |

#### `[wasm-template-repository]`

Controls where WASM templates are sourced from:

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `url` | String | Git repository URL for WASM templates | `"https://github.com/tari-project/wasm-template"` |
| `branch` | String | Git branch to use | `"main"` |
| `folder` | String | Subdirectory containing templates | `"wasm_templates"` |

### CLI Configuration Overrides

<!-- SOURCE: Verified against config.rs VALID_OVERRIDE_KEYS lines 10-17 -->
Override configuration via command line:

```bash
# Override project template repository
tari -e "project_template_repository.url=https://github.com/my-org/templates" create my-project

# Override template branch
tari -e "wasm_template_repository.branch=development" new my-template

# Multiple overrides
tari -e "project_template_repository.url=https://custom.git" \
     -e "wasm_template_repository.branch=custom" \
     create my-project
```

**Valid Override Keys**:
- `project_template_repository.url`
- `project_template_repository.branch`
- `project_template_repository.folder`
- `wasm_template_repository.url`
- `wasm_template_repository.branch`
- `wasm_template_repository.folder`

## Project Configuration

<!-- SOURCE: Verified against crates/cli/src/project/config.rs -->
### File Location

**Required location**: `{project_root}/tari.config.toml`

### Project Configuration Schema

```toml
# tari.config.toml (in project root)

[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

### Project Configuration Fields

<!-- SOURCE: Verified against project/config.rs lines 16-33 -->
#### `[network]`

Network and deployment configuration:

| Field | Type | Description | Default | Required |
|-------|------|-------------|---------|----------|
| `wallet-daemon-jrpc-address` | String (URL) | JSON-RPC endpoint for Tari Wallet Daemon | `"http://127.0.0.1:9000/"` | Yes |

**URL Format Requirements**:
- Must be valid HTTP/HTTPS URL
- Must include protocol (`http://` or `https://`)
- Must include port number
- Examples: `http://127.0.0.1:9000/`, `https://testnet-node:9000/`

### Network Configuration Examples

**Local Development**:
```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

**Remote Testnet**:
```toml
[network]
wallet-daemon-jrpc-address = "https://testnet-wallet.tari.com:9000/"
```

**Custom Port**:
```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9001/"
```

## Template Configuration

### Template Descriptor Schema

<!-- SOURCE: Verified against crates/cli/src/templates/collector.rs lines 136-152 -->
Every template requires a `template.toml` file:

```toml
# template.toml

name = "template-name"
description = "Human-readable template description"

# Optional extra configuration
[extra]
templates_dir = "templates"
wasm_templates = "true"
category = "tokens"
complexity = "beginner"
```

### Template Fields Reference

#### Required Fields

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `name` | String | Template identifier (converted to snake_case) | `"nft-template"` |
| `description` | String | Human-readable description shown during selection | `"A simple NFT template"` |

#### Optional `[extra]` Fields

| Field | Type | Description | Usage |
|-------|------|-------------|-------|
| `templates_dir` | String | Subdirectory containing template files | Project templates with nested WASM templates |
| `wasm_templates` | String | Comma-separated list of initial WASM templates | Auto-generate templates on project creation |
| `category` | String | Template category for organization | `"tokens"`, `"defi"`, `"governance"` |
| `complexity` | String | Difficulty level indicator | `"beginner"`, `"intermediate"`, `"advanced"` |

### Template Repository Structure

<!-- SOURCE: Verified against collector.rs template discovery logic -->
Templates are discovered by scanning for `template.toml` files:

```
template-repository/
├── basic-project/
│   ├── template.toml           # Project template descriptor
│   ├── Cargo.toml             # Project workspace config
│   └── src/                   # Template source files
├── templates/
│   ├── nft/
│   │   ├── template.toml      # WASM template descriptor  
│   │   ├── Cargo.toml         # Crate configuration
│   │   └── src/lib.rs         # Smart contract implementation
│   └── token/
│       ├── template.toml
│       ├── Cargo.toml
│       └── src/lib.rs
```

**Discovery Rules**:
- CLI recursively scans repository for `template.toml` files
- Template ID derived from directory name (converted to snake_case)
- Templates can be nested in any directory structure
- Both root-level and subdirectory templates are supported

## Command-Line Configuration

### Global Arguments

<!-- SOURCE: Verified against crates/cli/src/cli/arguments.rs lines 88-104 -->
Available for all commands:

```bash
tari [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `--base-dir <PATH>` | `-b` | Path | Base directory for CLI data | `~/.local/share/tari_cli` |
| `--config-file-path <PATH>` | `-c` | Path | Config file location | `~/.config/tari_cli/tari.config.toml` |
| `--config-overrides <KEY=VALUE>` | `-e` | String | Config overrides | None |

### Command-Specific Arguments

#### `create` Command

```bash
tari create [OPTIONS] <NAME>
```

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `<NAME>` | | String | Project name (converted to snake_case) | Required |
| `--template <TEMPLATE>` | `-t` | String | Project template ID | Interactive selection |
| `--target <PATH>` | | Path | Target directory | Current directory |

#### `new` Command

```bash
tari new [OPTIONS] <NAME>
```

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `<NAME>` | | String | Template name (converted to snake_case) | Required |
| `--template <TEMPLATE>` | `-t` | String | WASM template ID | Interactive selection |
| `--target <PATH>` | | Path | Target directory | Current directory |

#### `deploy` Command

<!-- SOURCE: Verified against crates/cli/src/cli/commands/deploy.rs lines 18-50 -->
```bash
tari deploy [OPTIONS] <TEMPLATE>
```

| Option | Short | Type | Description | Default |
|--------|-------|------|-------------|---------|
| `<TEMPLATE>` | | String | Template project to deploy | Required |
| `--account <ACCOUNT>` | `-a` | String | Account for deployment fees | Required |
| `--custom-network <NETWORK>` | `-c` | String | Custom network name | Project config default |
| `--yes` | `-y` | Flag | Auto-confirm deployment | `false` |
| `--max-fee <MAX_FEE>` | `-f` | u64 | Maximum deployment fee | Auto-estimated |
| `--project-folder <PATH>` | | Path | Project folder location | Current directory |

## Environment Variables

### CLI Data Directories

<!-- SOURCE: Verified against arguments.rs default directory functions -->
The CLI uses standard system directories:

**Linux/macOS**:
- **Data**: `~/.local/share/tari_cli/`
- **Config**: `~/.config/tari_cli/`
- **Templates**: `~/.local/share/tari_cli/template_repositories/`

**Windows**:
- **Data**: `%APPDATA%\tari_cli\`
- **Config**: `%APPDATA%\tari_cli\`
- **Templates**: `%APPDATA%\tari_cli\template_repositories\`

### Build Environment Variables

These affect WASM compilation and deployment:

| Variable | Effect | Example |
|----------|--------|---------|
| `RUST_LOG` | Enable debug logging | `RUST_LOG=debug tari create my-project` |
| `CARGO_TARGET_DIR` | Override build directory | `CARGO_TARGET_DIR=./build tari deploy` |
| `RUSTFLAGS` | Pass flags to Rust compiler | `RUSTFLAGS="-C target-cpu=native"` |

### Network Environment Variables

<!-- SOURCE: Verified against CI configuration -->
Used in CI/CD and testing:

| Variable | Effect | Example |
|----------|--------|---------|
| `TARI_NETWORK` | Set target network | `TARI_NETWORK=testnet` |
| `TARI_TARGET_NETWORK` | Set deployment target | `TARI_TARGET_NETWORK=localnet` |

## Validation and Errors

### Configuration Validation

The CLI validates all configuration at startup:

**URL Validation**:
```rust
// Must be valid URL with protocol and port
Url::parse("http://127.0.0.1:9000/").unwrap()
```

**Override Key Validation**:
```rust
// Only specific keys are allowed for overrides
const VALID_OVERRIDE_KEYS: &[&str] = &[
    "project_template_repository.url",
    "wasm_template_repository.branch",
    // ... other valid keys
];
```

### Common Configuration Errors

**Invalid URL Format**:
```
Error: URL parsing error: invalid port number
```
**Solution**: Ensure URL includes protocol and valid port

**Invalid Override Key**:
```
Error: Invalid key: invalid.override.key
```
**Solution**: Use only valid override keys from reference

**Missing Configuration File**:
```
Failed to load project config file (at /path/to/tari.config.toml)
```
**Solution**: Create `tari.config.toml` in project root

**Invalid TOML Syntax**:
```
Error: Failed to deserialize TOML
```
**Solution**: Validate TOML syntax and field names

## Configuration Examples

### Development Setup

Complete configuration for local development:

```toml
# ~/.config/tari_cli/tari.config.toml
[project-template-repository]
url = "https://github.com/tari-project/wasm-template"
branch = "main"
folder = "project_templates"

[wasm-template-repository]
url = "https://github.com/tari-project/wasm-template"  
branch = "main"
folder = "wasm_templates"
```

```toml
# project_root/tari.config.toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

### Multi-Environment Setup

Different configurations for various environments:

**Development** (`tari.config.dev.toml`):
```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

**Testnet** (`tari.config.testnet.toml`):
```toml
[network]
wallet-daemon-jrpc-address = "https://testnet-wallet:9000/"
```

**Production** (`tari.config.prod.toml`):
```toml
[network]
wallet-daemon-jrpc-address = "https://mainnet-wallet.secure.com:9000/"
```

### Custom Template Repository

Using organization-specific templates:

```toml
# ~/.config/tari_cli/tari.config.toml
[project-template-repository]
url = "https://github.com/my-org/tari-templates"
branch = "stable"
folder = "projects"

[wasm-template-repository]
url = "https://github.com/my-org/tari-templates"
branch = "stable"  
folder = "contracts"
```

### CI/CD Configuration

Minimal configuration for automated environments:

```bash
# Environment-based configuration
export TARI_NETWORK=testnet
export CARGO_TARGET_DIR=/tmp/build

# CLI with overrides
tari -b /tmp/tari_cli \
     -e "wasm_template_repository.branch=ci-stable" \
     deploy --account ci-account --yes my_template
```

## Migration and Upgrades

### Configuration Version Compatibility

The CLI maintains backward compatibility for configuration files:

- **Missing fields**: Filled with defaults
- **Extra fields**: Ignored gracefully  
- **Deprecated fields**: Warnings displayed

### Migrating Between Versions

When upgrading Tari CLI:

1. **Backup existing config**: Copy current `tari.config.toml` files
2. **Check deprecation warnings**: Note any warnings on startup
3. **Update templates**: Refresh template repositories after upgrade
4. **Validate deployment**: Test deployment on development network

---

**Need help with configuration?** Check our [Common Issues](../04-troubleshooting/common-issues.md#project-configuration-issues) or [CLI Commands Reference](cli-commands.md) for specific usage examples.
