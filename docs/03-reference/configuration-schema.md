---
title: Configuration Schema Reference
description: Complete reference for all Tari CLI configuration options and file formats
last_updated: 2026-04-14
version: "0.14"
verified_against: crates/cli/src/cli/config.rs, crates/cli/src/project/config.rs
audience: users
---

# Configuration Schema Reference

> **Complete reference** for all Tari CLI configuration files and options

## Configuration Hierarchy

Settings are resolved in this order (highest priority first):

1. **Command-line flags** (`--wallet-daemon-url`, `--metadata-server-url`, etc.)
2. **CLI overrides** (`-e KEY=VALUE`)
3. **Project configuration** (`tari.config.toml` in project or git root)
4. **Global CLI configuration** (`~/.config/tari_cli/tari.config.toml`)
5. **Built-in defaults**

---

## Global CLI Configuration

### File Location

**Default**: `~/.config/tari_cli/tari.config.toml`

**Custom**: use `--config-file-path` or `-c`

### Schema

```toml
# ~/.config/tari_cli/tari.config.toml

[template-repository]
url = "https://github.com/tari-project/wasm-template"
branch = "main"
folder = "wasm_templates"

# Optional
# wallet-daemon-url = "http://127.0.0.1:9000/json_rpc"
# metadata-server-url = "http://localhost:3000"
# default-account = "myaccount"
```

### Fields

#### `[template-repository]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | String | `https://github.com/tari-project/wasm-template` | Git repository URL for templates |
| `branch` | String | `main` | Git branch |
| `folder` | String | `wasm_templates` | Subdirectory containing templates |

#### Top-level optional fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `wallet-daemon-url` | URL | None | Global wallet daemon JSON-RPC URL |
| `metadata-server-url` | URL | None | Global metadata server URL |
| `default-account` | String | None | Default wallet account for publishing |

### CLI Overrides (`-e`)

Valid override keys:

| Key | Example |
|-----|---------|
| `template_repository.url` | `https://github.com/my-org/templates` |
| `template_repository.branch` | `development` |
| `template_repository.folder` | `my_templates` |
| `default_account` | `myaccount` |
| `wallet_daemon_url` | `http://localhost:12008/json_rpc` |
| `metadata_server_url` | `http://community.example.com` |

```bash
tari -e "wallet_daemon_url=http://localhost:12008/json_rpc" publish
```

---

## Project Configuration

### File Location

`tari.config.toml` in the project root or git repository root. Created with `tari init`, `tari config init`, or automatically by the wizard.

### Schema

```toml
# tari.config.toml

[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:5100/json_rpc"

# Optional
# default-account = "myaccount"
# metadata-server-url = "http://localhost:3000"
# template-address = "template_abc123..."
```

### Fields

#### `[network]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `wallet-daemon-jrpc-address` | URL | `http://127.0.0.1:5100/json_rpc` | Wallet daemon JSON-RPC endpoint |

#### Top-level optional fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default-account` | String | None | Default wallet account |
| `metadata-server-url` | URL | None | Metadata server URL |
| `template-address` | Address | None | Template address (saved automatically by `tari publish`) |

### Managing Project Configuration

```bash
# Create default config
tari config init

# Set wallet daemon URL
tari config set network.wallet-daemon-jrpc-address http://localhost:12008/json_rpc

# Set metadata server
tari config set metadata_server_url http://community.example.com

# View current config
tari config show
```

---

## Template Metadata Configuration

Template metadata is stored in `Cargo.toml` under `[package.metadata.tari-template]`. It is read at build time by `tari_ootle_template_build` and encoded as CBOR.

### Schema

```toml
[package]
name = "my-template"
version = "1.0.0"
description = "A fungible token with mint/burn/transfer"
license = "BSD-3-Clause"
repository = "https://github.com/example/my-template"

[package.metadata.tari-template]
tags = ["token", "fungible", "defi"]
category = "token"
documentation = "https://docs.example.com/"
homepage = "https://example.com/"
logo_url = "https://example.com/logo.png"

[package.metadata.tari-template.extra]
audit = "https://example.com/audit-report"
```

### Fields

Fields from `[package]` (read automatically):

| Field | Source | Description |
|-------|--------|-------------|
| `name` | `[package].name` | Template name (required) |
| `version` | `[package].version` | Template version (required) |
| `description` | `[package].description` | Description |
| `license` | `[package].license` | License identifier |
| `repository` | `[package].repository` | Repository URL |

Fields from `[package.metadata.tari-template]`:

| Field | Type | Description |
|-------|------|-------------|
| `tags` | Array of strings | Searchable tags |
| `category` | String | Template category |
| `documentation` | String | Documentation URL |
| `homepage` | String | Homepage URL |
| `logo_url` | String | Logo/icon image URL |

Fields from `[package.metadata.tari-template.extra]`:

Arbitrary key-value pairs (string values only).

### Setting Up Metadata

```bash
# One-step: initialise project config AND template metadata
tari init

# Or just template metadata
tari template init

# Non-interactive
tari init -y --tags token,defi --category token --logo-url https://example.com/logo.png
```

### Inspecting Metadata

```bash
# Human-readable table
tari metadata inspect

# JSON output
tari metadata inspect --json
```

---

## Template Descriptor

Template repositories use `template.toml` to describe each starter template:

```toml
name = "fungible-token"
description = "A standard fungible token with mint/burn/transfer"

[extra]
category = "tokens"
complexity = "beginner"
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Template identifier |
| `description` | Yes | Shown during interactive selection |
| `[extra]` | No | Arbitrary metadata |

---

## Data Directories

| Directory | macOS/Linux | Purpose |
|-----------|-------------|---------|
| Data | `~/.local/share/tari_cli/` | CLI data and cached repos |
| Config | `~/.config/tari_cli/` | Global config file |
| Templates | `~/.local/share/tari_cli/template_repositories/` | Cloned template repos |

---

For command usage, see the [CLI Commands Reference](cli-commands.md).
