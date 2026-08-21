---
title: CLI Commands Reference
description: Complete reference for all Tari CLI commands, arguments, and options
last_updated: 2026-08-20
version: "0.15"
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
| `--config-overrides <KEY=VALUE>` | `-e` | Config file overrides (e.g. `networks.esmeralda.wallet-daemon-url=...`) | None |
| `--network <NETWORK>` | `-n` | Active network (`esmeralda`, `localnet`, `igor`, `nextnet`, `stagenet`, `mainnet`). Overrides project and global `default-network` | Project / global default |
| `--api-key <API_KEY>` | | Wallet daemon API key, sent as a bearer token. Also read from `TARI_WALLET_DAEMON_API_KEY` | `$TARI_WALLET_DAEMON_API_KEY` |

### Wallet daemon authentication

Commands that talk to the wallet daemon (`publish`, `template publish`, and `metadata publish --signed`) authenticate with an **API key** issued by the wallet daemon. The key is sent as an `Authorization: Bearer` token on every JSON-RPC request — there is no interactive login.

Provide the key with the `--api-key` flag or the `TARI_WALLET_DAEMON_API_KEY` environment variable (the flag takes precedence):

```bash
export TARI_WALLET_DAEMON_API_KEY="<your-api-key>"
tari publish -a myaccount

# or per-invocation
tari publish -a myaccount --api-key "<your-api-key>"
```

For security, the API key is **never** read from or written to a config file.

The key must be minted with at least these permissions for the CLI to work:

| Permission | Used for |
|------------|----------|
| `templates:read` | Reading template state |
| `templates:create` | Publishing templates and signing metadata |
| `accounts:read` | Resolving the fee account and checking its balance |
| `transactions:read` | Waiting on the publish transaction result to confirm it |

If the wallet daemon has authentication disabled, the API key may be omitted.

## Commands Overview

| Command | Alias | Purpose |
|---------|-------|---------|
| [`init`](#init) | | Initialise project config and template build.rs |
| [`create`](#create) | `new` | Create a new template crate from a starter template |
| [`build`](#build) | | Build the template WASM binary |
| [`lint`](#lint) | `lints`, `check` | Check the template crate for Rust lints, binary size and metadata issues |
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

## `lint`

Checks a template crate for issues that hurt the published WASM binary or the developer experience. Every check except the Rust lints prints the exact fix, and most can be applied automatically with `--fix`.

```bash
tari lint [PATH] [OPTIONS]
# aliases: tari lints, tari check
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `[PATH]` | Path | `.` | Path to the template crate directory (or its `Cargo.toml`) |

| Option | Short | Description |
|--------|-------|-------------|
| `--fix` | | Apply the suggested fix for every issue that can be fixed automatically |
| `--no-clippy` | | Skip the `cargo clippy` run and only check the manifests |
| `--deny-warnings` | `-D` | Exit non-zero on warnings and suggestions, not just errors |

### Checks

| Code | Severity | Checks | `--fix` |
|------|----------|--------|---------|
| `rust::clippy` | error / warning | Runs `cargo clippy --target wasm32-unknown-unknown` and reports its diagnostics | Runs `cargo clippy --fix` |
| `cargo::crate-type` | error / warning | `[lib] crate-type` is exactly `["cdylib"]` — extra types such as `rlib` are compiled and linked in, bloating the binary | ✅ |
| `cargo::release-profile` | warning | `[profile.release]` declares the size optimizations (`opt-level`, `lto`, `codegen-units`, `panic`, `strip`) | ✅ |
| `cargo::test-runtime-profile` | suggestion | Wasmer/Cranelift are optimized in dev builds, so template tests are not ~10x slower. Only checked when the crate has tests | ✅ |
| `template::metadata` | warning / suggestion | `description` and `[package.metadata.tari-template]` fields are filled in | ❌ — only you know the values; run `tari template init` |
| `template::metadata-build` | warning | `tari_ootle_template_build` is a build dependency and `build.rs` calls `TemplateMetadataBuilder` | ✅ (unless an unrelated `build.rs` already exists) |

Cargo only honours `[profile.*]` in the workspace root manifest, so profile checks read — and `--fix` writes — the workspace root `Cargo.toml` when the template is a workspace member.

`tari build` and `tari publish` already pass the release profile settings via `cargo --config`, so `cargo::release-profile` matters for anyone building the crate with plain `cargo build --release`.

### Exit codes

`0` when no errors were found, `1` when any error was found (or any warning with `--deny-warnings`) — suitable for CI.

### Example

```bash
tari lint
# ⚠️ warning[cargo::crate-type]: `crate-type` also contains `rlib` — every extra crate type is
#    compiled and linked in, bloating the published binary
#    --> Cargo.toml [lib]
#    help:
#        In Cargo.toml, keep only the dynamic library:
#
#        [lib]
#        crate-type = ["cdylib"]
#    (fixable with `tari lint --fix`)
#
# Summary: 0 error(s), 1 warning(s), 0 suggestion(s)

# Apply everything that can be fixed automatically
tari lint --fix

# Fail CI on any finding, without needing a clippy install
tari lint --deny-warnings --no-clippy
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
| `-n, --network` | Network | Project/global default | Active network (overrides config) |
| `-c, --custom-network` | String | Config default | Custom network name |
| `-y, --yes` | Flag | `false` | Skip confirmation prompt |
| `-f, --max-fee` | u64 | Auto-estimated | Maximum fee in microtari |
| `--binary, --bin` | Path | *builds if not set* | Path to pre-compiled WASM binary |
| `--wallet-daemon-url` | URL | `[networks.<active>].wallet-daemon-url` | Wallet daemon JSON-RPC URL |
| `--api-key` | String | `$TARI_WALLET_DAEMON_API_KEY` | Wallet daemon API key (bearer token) |
| `--publish-metadata` | Flag | `false` | Auto-submit metadata to server after publishing |
| `--metadata-server-url` | URL | `[networks.<active>].metadata-server-url` | Metadata server URL (with `--publish-metadata`) |

Before publishing, the CLI verifies the wallet daemon is on the same network as the active CLI network and aborts with an error if they differ.

After publishing:
- The template address is saved under `[networks.<active>].template-address` in `tari.config.toml` (so `tari metadata publish` can omit `--template-address`)
- If metadata is detected and `--publish-metadata` is not set, you will be prompted to publish it
- If a template address already exists for the active network (republishing), a warning is shown

### Example

```bash
# Build and publish (uses default-network from config)
tari publish -a myaccount -y

# Publish and auto-submit metadata
tari publish -a myaccount --publish-metadata

# Override the active network
tari --network localnet publish -a myaccount
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

The build dependency is pinned to the latest release found on the crates.io index. If crates.io cannot be reached the CLI says so and falls back to the latest version known at CLI release time, so `tari template init` still works offline. An existing `tari_ootle_template_build` declaration is never re-pinned.

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
| `-n, --network` | Network | Project/global default | Active network (overrides config) |
| `-t, --template-address` | Address | `[networks.<active>].template-address` | Template address. If omitted, uses the address saved by `tari publish` |
| `--metadata-server-url` | URL | `[networks.<active>].metadata-server-url` | Metadata server URL |
| `--max-retries` | u32 | `6` | Max retry attempts for 404 (template not yet synced) |
| `--signed` | Flag | `false` | Use author-signed submission via wallet daemon |
| `--key-index` | u64 | `0` | Derived account key index (with `--signed`) |
| `--wallet-daemon-url` | URL | `[networks.<active>].wallet-daemon-url` | Wallet daemon URL (with `--signed`) |
| `--api-key` | String | `$TARI_WALLET_DAEMON_API_KEY` | Wallet daemon API key (bearer token, with `--signed`) |

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
tari config set networks.localnet.wallet-daemon-url http://localhost:12008/json_rpc
tari config set networks.esmeralda.metadata-server-url http://community.example.com
tari config set default-network localnet
tari config set default-account myaccount
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

The active network is resolved first, then per-setting values are read from that network's section.

**Active network** (highest priority first): `--network` → project `default-network` → global `default-network` → `esmeralda`.

**Per-setting** (highest priority first):

| Setting | CLI flag | Project config | Global config | Default |
|---------|----------|---------------|---------------|---------|
| Wallet daemon URL | `--wallet-daemon-url` | `networks.<active>.wallet-daemon-url` | `networks.<active>.wallet-daemon-url` | `http://127.0.0.1:5100/json_rpc` |
| Metadata server URL | `--metadata-server-url` | `networks.<active>.metadata-server-url` | `networks.<active>.metadata-server-url` | esmeralda → `https://ootle.tari.com/community-templates`, localnet → `http://localhost:3000/`, others → none |
| Template address | `--template-address` | `networks.<active>.template-address` | — | — |
| Account | `--account` | `default-account` | `default-account` | Wallet daemon default |
| Wallet daemon API key | `--api-key` | — (never stored in config) | — (never stored in config) | `TARI_WALLET_DAEMON_API_KEY` env var |

---

For configuration file details, see the [Configuration Schema Reference](configuration-schema.md).
