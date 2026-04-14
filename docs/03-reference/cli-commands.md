---
title: CLI Commands Reference
description: Complete reference for all Tari CLI commands, arguments, and options
last_updated: 2026-04-14
version: "0.14"
verified_against: crates/cli/src/cli/command.rs, command implementations
audience: users
---

# CLI Commands Reference

> **Complete reference** for all Tari CLI commands, arguments, and usage patterns

## Global Options

Available for all commands:

```bash
tari [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--base-dir <PATH>` | `-b` | Base directory for CLI data | `~/.local/share/tari_cli` |
| `--config-file-path <PATH>` | `-c` | Config file location | `~/.config/tari_cli/tari.config.toml` |
| `--config-overrides <KEY=VALUE>` | `-e` | Config file overrides | None |

## Commands Overview

| Command | Alias | Purpose |
|---------|-------|---------|
| [`init`](#init) | | Initialise project config and template build.rs |
| [`create`](#create) | `new` | Create a new template crate from a starter template |
| [`build`](#build) | | Build the template WASM binary |
| [`publish`](#publish) | `deploy` | Publish a template to the network |
| [`template`](#template) | | Template metadata tooling (init, inspect, publish) |
| [`metadata`](#metadata) | | Metadata server operations (inspect, publish) |
| [`config`](#config) | | Manage project configuration |
| *(no command)* | | [Interactive setup wizard](#wizard) |

---

## `init`

Initialises the project config (`tari.config.toml`) and template `build.rs` in a single step. Combines `tari config init` and `tari template init`.

```bash
tari init [OPTIONS] [PATH]
```

| Argument / Option | Type | Default | Description |
|-------------------|------|---------|-------------|
| `[PATH]` | Path | `.` | Path to the template crate directory (containing Cargo.toml) |
| `--description` | String | *prompted if missing* | Template description (written to `[package].description`) |
| `--tags` | String (comma-separated) | *prompted* | Tags (e.g. "token,fungible,defi") |
| `--category` | String | *prompted* | Template category |
| `--documentation` | String | *prompted* | Documentation URL |
| `--homepage` | String | *prompted* | Homepage URL |
| `--logo-url` | String | *prompted* | Logo URL |
| `-y, --non-interactive` | Flag | `false` | Skip interactive prompts |

### Example

```bash
# Interactive — prompts for metadata fields
tari init

# Non-interactive with metadata
tari init -y --tags token,defi --category token
```

---

## `create`

Creates a new Tari template crate from a starter template. Alias: `new`.

```bash
tari create [OPTIONS] [NAME]
```

| Argument / Option | Type | Default | Description |
|-------------------|------|---------|-------------|
| `[NAME]` | String | *prompted* | Name of the new template crate (converted to snake_case). If omitted, you will be prompted |
| `-t, --template` | String | *prompted* | Template to use (e.g. "fungible", "meme_coin"). Prompted if not set |
| `-o, --output <PATH>` | Path | Current directory | Directory where the new crate will be created |
| `--skip-init` | Flag | `false` | Skip git repository initialisation |
| `--skip-metadata` | Flag | `false` | Skip automatic template metadata initialisation |
| `-v, --verbose` | Flag | `false` | Enable verbose output |

### Example

```bash
# Interactive — prompts for name and template
tari create

# Specify everything
tari create my-token --template fungible -o ~/projects/
```

---

## `build`

Builds the template WASM binary and reports the metadata CBOR file path (if present).

```bash
tari build [PATH]
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `[PATH]` | Path | `.` | Path to the template crate directory |

### Example

```bash
tari build
# ✅ WASM binary: target/wasm32.../release/my_token.wasm (42.3 KB)
# 📄 Metadata:    target/wasm32.../release/build/.../out/template_metadata.cbor
```

---

## `publish`

Publishes a template to the Tari network. Alias: `deploy`. Delegates to `tari template publish`.

```bash
tari publish [OPTIONS] [PATH]
```

| Argument / Option | Type | Default | Description |
|-------------------|------|---------|-------------|
| `[PATH]` | Path | `.` | Path to the template crate directory |
| `-a, --account` | String | Config or wallet default | Account for publishing fees |
| `-c, --custom-network` | String | Config default | Custom network name |
| `-y, --yes` | Flag | `false` | Skip confirmation prompt |
| `-f, --max-fee` | u64 | Auto-estimated | Maximum fee in microtari |
| `--binary, --bin` | Path | *builds if not set* | Path to pre-compiled WASM binary |
| `--wallet-daemon-url` | URL | Config default | Wallet daemon JSON-RPC URL |
| `--publish-metadata` | Flag | `false` | Auto-submit metadata to server after publishing |
| `--metadata-server-url` | URL | Config or `localhost:3000` | Metadata server URL (with `--publish-metadata`) |

After publishing:
- The template address is saved to `tari.config.toml` (so `tari metadata publish` can omit `--template-address`)
- If metadata is detected and `--publish-metadata` is not set, you will be prompted to publish it
- If a template address already exists in config (republishing), a warning is shown

### Example

```bash
# Build and publish
tari publish -a myaccount -y

# Publish and auto-submit metadata
tari publish -a myaccount --publish-metadata
```

---

## `template`

Template metadata tooling.

### `template init`

Sets up an existing template crate for metadata generation. Alias: `template init-metadata`.

```bash
tari template init [OPTIONS] [PATH]
```

| Argument / Option | Type | Default | Description |
|-------------------|------|---------|-------------|
| `[PATH]` | Path | `.` | Path to template crate directory |
| `--description` | String | *prompted if missing* | Template description (written to `[package].description`) |
| `--tags` | String (comma-separated) | *prompted* | Tags (e.g. "token,fungible,defi") |
| `--category` | String | *prompted* | Template category |
| `--documentation` | String | *prompted* | Documentation URL |
| `--homepage` | String | *prompted* | Homepage URL |
| `--logo-url` | String | *prompted* | Logo URL |
| `-y, --non-interactive` | Flag | `false` | Skip interactive prompts |

Adds `tari_ootle_template_build` to `[build-dependencies]`, creates `build.rs`, and writes a `[package.metadata.tari-template]` section to `Cargo.toml`.

### `template inspect`

Inspects a template metadata CBOR file. Alias: `template inspect-metadata`.

If the built metadata doesn't match `Cargo.toml`, you will be prompted to rebuild.

```bash
tari template inspect [OPTIONS] [PATH]
```

| Argument / Option | Type | Default | Description |
|-------------------|------|---------|-------------|
| `[PATH]` | Path | *searches build output* | Path to metadata CBOR file |
| `--project-dir` | Path | `.` | Project directory to search (when path not given) |
| `--json` | Flag | `false` | Output as JSON |

### `template publish`

Publishes a template with its metadata hash. Same options as [`publish`](#publish).

---

## `metadata`

Template metadata server operations.

### `metadata inspect`

Alias for [`template inspect`](#template-inspect).

### `metadata publish`

Publishes template metadata to a community metadata server.

```bash
tari metadata publish [OPTIONS] [-t <TEMPLATE_ADDRESS>]
```

| Argument / Option | Type | Default | Description |
|-------------------|------|---------|-------------|
| `[PATH]` | Path | `.` | Path to template crate directory |
| `-t, --template-address` | Address | From config | Template address. If omitted, uses the address saved by `tari publish` |
| `--metadata-server-url` | URL | Config or `localhost:3000` | Metadata server URL |
| `--max-retries` | u32 | `6` | Max retry attempts for 404 (template not yet synced) |
| `--signed` | Flag | `false` | Use author-signed submission via wallet daemon |
| `--key-index` | u64 | `0` | Derived account key index (with `--signed`) |
| `--wallet-daemon-url` | URL | Config default | Wallet daemon URL (with `--signed`) |

#### Hash-verified (default)

POSTs raw CBOR metadata. The server verifies the hash matches the on-chain `metadata_hash`. Requires the template to have been published with a metadata hash.

```bash
tari metadata publish -t template_bce07f...
```

#### Author-signed (`--signed`)

Signs metadata via the wallet daemon (Schnorr signature). Allows updating metadata without republishing on-chain. No secret keys touch the CLI.

```bash
tari metadata publish -t template_bce07f... --signed --key-index 0
```

Both flows retry with exponential backoff on 404 (template not yet synced by the server).

---

## `config`

Manage project configuration (`tari.config.toml`).

### `config init`

Creates a `tari.config.toml` with defaults in the project root (or git repo root).

```bash
tari config init
```

### `config set`

Sets a configuration value.

```bash
tari config set <KEY> <VALUE>
```

Examples:
```bash
tari config set network.wallet-daemon-jrpc-address http://localhost:12008/json_rpc
tari config set metadata_server_url http://community.example.com
tari config set default_account myaccount
```

### `config get`

```bash
tari config get <KEY>
```

### `config show`

Displays the full configuration file.

```bash
tari config show
```

---

## Wizard

Running `tari` with no command launches an interactive setup wizard that walks you through:

1. Creating or detecting a template crate
2. Setting up project configuration (`tari.config.toml`)
3. Initialising template metadata

---

## Configuration Resolution

Settings are resolved in priority order (highest first):

| Setting | CLI flag | Project config | Global config | Default |
|---------|----------|---------------|---------------|---------|
| Wallet daemon URL | `--wallet-daemon-url` | `network.wallet-daemon-jrpc-address` | `wallet_daemon_url` | `http://127.0.0.1:9000/json_rpc` |
| Metadata server URL | `--metadata-server-url` | `metadata-server-url` | `metadata_server_url` | `http://localhost:3000` |
| Account | `--account` | `default_account` | `default_account` | Wallet daemon default |

---

For configuration file details, see the [Configuration Schema Reference](configuration-schema.md).
