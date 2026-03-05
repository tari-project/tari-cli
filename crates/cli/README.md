# Tari Ootle CLI

The official command-line interface for developing and publishing Tari smart contract templates on the Tari Ootle (Layer-2) network.

## Installation

```bash
cargo install tari-ootle-cli
```

Or build from source:

```bash
cargo build --release
```

The binary is named `tari`.

## Commands

### `tari create`

Creates a new workspace for a Tari template project. Scaffolds a Cargo workspace with project configuration and lets you choose from available project templates.

```bash
tari create my-project
```

Aliases: `new`

### `tari add`

Generates and adds a new Tari WASM template crate. Can be used inside an existing workspace or standalone.

```bash
tari add MyTemplate
```

Aliases: `generate`, `gen`

### `tari publish`

Publishes a compiled Tari template to a network. Handles WASM compilation, fee estimation, balance verification, and submission.

```bash
tari publish --account myaccount MyTemplate
```

Aliases: `deploy`

Options:
- `-a, --account` - Account to use for publishing fees
- `-c, --custom-network` - Custom network name (must match project config)
- `-y, --yes` - Skip confirmation prompt
- `-f, --max-fee` - Maximum fee limit
- `--project-folder` - Project folder path (defaults to current directory)
- `--binary` - Path to a pre-compiled WASM binary (skips build step)

## Configuration

The CLI uses two configuration layers:

- **Global config** (`~/.local/share/tari_cli/tari.config.toml`) - Template repositories and default account
- **Project config** (`tari.config.toml` in project root) - Network settings and project-level defaults

Override config values at runtime with `-e`:

```bash
tari -e "default_account=myaccount" publish MyTemplate
```

## Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- A running [Tari Wallet Daemon](https://github.com/tari-project/tari-dan) for publishing
