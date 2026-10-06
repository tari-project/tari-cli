---
title: Configuration Schema Reference
description: Complete reference for all Tari CLI configuration options and file formats
last_updated: 2026-04-22
version: "0.15"
verified_against: crates/cli/src/cli/config.rs, crates/cli/src/project/config.rs
audience: users
---

# Configuration Schema Reference

> **Complete reference** for all Tari CLI configuration files and options

## Configuration Hierarchy

Both the global CLI config and the project config are organised by **network** (`esmeralda`, `localnet`, `igor`, `nextnet`, `stagenet`, `mainnet`). Each command resolves an **active network** and then reads `metadata-server-url` and `template-address` (and, from the global config only, `wallet-daemon-url`) from the matching `[networks.<name>]` section.

### Active network resolution (highest priority first)

1. `--network <name>` (`-n`) on the command line
2. `default-network` in the project `tari.config.toml`
3. `default-network` in the global CLI config
4. `esmeralda` (built-in default)

### Per-setting resolution (highest priority first)

1. **Command-line flags** (`--wallet-daemon-url`, `--metadata-server-url`, etc.; `TARI_WALLET_DAEMON_URL` for the wallet daemon URL)
2. **CLI overrides** (`-e KEY=VALUE`)
3. **Project configuration** — `[networks.<active>]` in `tari.config.toml` (not used for `wallet-daemon-url`)
4. **Global CLI configuration** — `[networks.<active>]` in `~/.config/tari_cli/tari.config.toml`
5. **Built-in defaults**

### Wallet daemon URL

`wallet-daemon-url` is **not** read from the project `tari.config.toml`: a cloned repo could otherwise redirect your wallet API key. If present there it is ignored with a warning, and `tari config set ...wallet-daemon-url` errors. Resolution (highest priority first):

1. `--wallet-daemon-url` flag or `TARI_WALLET_DAEMON_URL` env var
2. `[networks.<active>].wallet-daemon-url` in the global CLI config (also settable with `-e networks.<name>.wallet-daemon-url=...`)
3. `http://127.0.0.1:5100/json_rpc`

The CLI refuses to send an API key over plain `http` to a non-loopback host.

---

## Global CLI Configuration

### File Location

**Default**: `~/.config/tari_cli/tari.config.toml`

**Custom**: use `--config-file-path` or `-c`

### Schema

```toml
# ~/.config/tari_cli/tari.config.toml

default-network = "esmeralda"
# default-account = "myaccount"

[template-repository]
url = "https://github.com/tari-project/wasm-template"
branch = "main"
folder = "wasm_templates"

[networks.esmeralda]
wallet-daemon-url = "http://127.0.0.1:5100/json_rpc"
metadata-server-url = "https://ootle.tari.com/community-templates"

[networks.localnet]
wallet-daemon-url = "http://127.0.0.1:5100/json_rpc"
metadata-server-url = "http://localhost:3000/"
```

### Fields

#### Top-level

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default-network` | Network | `esmeralda` | Used when no `--network` flag and no project default-network |
| `default-account` | String | None | Default wallet account for publishing |

#### `[template-repository]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | String | `https://github.com/tari-project/wasm-template` | Git repository URL for templates |
| `branch` | String | `main` | Git branch |
| `folder` | String | `wasm_templates` | Subdirectory containing templates |

#### `[networks.<name>]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `wallet-daemon-url` | URL | `http://127.0.0.1:5100/json_rpc` | Wallet daemon JSON-RPC endpoint |
| `metadata-server-url` | URL | esmeralda → `https://ootle.tari.com/community-templates`, localnet → `http://localhost:3000/`, others → none | Metadata server URL |

### CLI Overrides (`-e`)

Valid override keys:

| Key | Example |
|-----|---------|
| `template_repository.url` | `https://github.com/my-org/templates` |
| `template_repository.branch` | `development` |
| `template_repository.folder` | `my_templates` |
| `default_account` | `myaccount` |
| `default_network` | `localnet` |
| `networks.<name>.wallet-daemon-url` | `http://localhost:12008/json_rpc` |
| `networks.<name>.metadata-server-url` | `http://community.example.com` |

```bash
tari -e "networks.esmeralda.wallet-daemon-url=http://localhost:12008/json_rpc" publish
```

---

## Project Configuration

### File Location

`tari.config.toml` in the project root or git repository root. Created with `tari init`, `tari config init`, or automatically by the wizard.

### Schema

```toml
# tari.config.toml

default-network = "esmeralda"
# default-account = "myaccount"

[networks.esmeralda]
metadata-server-url = "https://ootle.tari.com/community-templates"
# template-address = "template_abc123..."  # written automatically by `tari publish`

[networks.localnet]
metadata-server-url = "http://localhost:3000/"
```

### Fields

#### Top-level

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default-network` | Network | `esmeralda` | Active network when no `--network` flag is set |
| `default-account` | String | None | Default wallet account |

#### `[networks.<name>]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `metadata-server-url` | URL | None | Metadata server URL for this network |
| `template-address` | Address | None | Most recently published template address (written automatically by `tari publish`) |

`<name>` is a value of the `Network` enum: `mainnet`, `stagenet`, `nextnet`, `localnet`, `igor`, `esmeralda`.

### Managing Project Configuration

```bash
# Create default config (esmeralda + localnet sections)
tari config init

# Change the default network
tari config set default-network localnet

# Set metadata server for esmeralda
tari config set networks.esmeralda.metadata-server-url http://community.example.com

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
