---
name: tari-cli
description: Develop, build, lint and publish Tari Ootle smart contract templates with the `tari` CLI. Use when working in a Tari template crate, or when asked to create, build, lint, publish, or inspect the metadata of a Tari template.
---

# Tari CLI

`tari` is the development tool for Tari Ootle templates — WASM smart contracts written in Rust.
A template is a normal Rust crate compiled to `wasm32-unknown-unknown`, with a `build.rs` that
generates a metadata CBOR file, published to an Ootle network through a **wallet daemon**.

Full docs: <https://tari-project.github.io/tari-cli/>

## Rules for agents

1. **Never run `tari` with no subcommand.** It launches an interactive wizard and will hang
   waiting on input. Always pass a subcommand.
2. **Always pass the non-interactive flag** on commands that prompt: `-y/--non-interactive` for
   `init` and `template init`, `-y/--yes` for `publish`. Supply every value the command would
   otherwise prompt for, otherwise the run blocks.
3. **Never invent or write an API key into a file.** The wallet daemon key comes from the
   `TARI_WALLET_DAEMON_API_KEY` environment variable (or `--api-key`) and is never stored in
   config. Ask the user for it rather than guessing.
4. **Publishing spends money and is irreversible.** `tari publish` submits an on-chain
   transaction that costs fees. Confirm with the user before running it, especially with
   `--network mainnet`.
5. Prefer `tari build` / `tari lint` over raw `cargo` — they inject the size-optimizing release
   profile and check things `cargo` does not.

## Prerequisites

```bash
cargo install tari-ootle-cli     # binary is called `tari`
rustup target add wasm32-unknown-unknown
```

Publishing additionally needs a running wallet daemon on the target network.

## Typical workflow

```bash
tari create my_token -t fungible   # scaffold a new template crate (or `tari init` in an existing one)
cd my_token
tari lint --fix                    # fix what can be fixed automatically
tari build                         # compile the WASM + metadata
tari publish -a myaccount -y       # publish on-chain (costs fees)
tari metadata publish              # submit metadata to the community server
```

## Commands

`tari [GLOBAL OPTIONS] <COMMAND> [OPTIONS]`

### Global options

| Option | Description |
|--------|-------------|
| `-n, --network <NETWORK>` | `esmeralda`, `localnet`, `igor`, `nextnet`, `stagenet`, `mainnet`. Overrides project/global `default-network` (default `esmeralda`). |
| `--api-key <KEY>` | Wallet daemon bearer token. Also read from `TARI_WALLET_DAEMON_API_KEY`. |
| `-c, --config-file-path <PATH>` | Global config file (default `~/.config/tari_cli/tari.config.toml`). |
| `-b, --base-dir <PATH>` | CLI data directory (default `~/.local/share/tari_cli`). |
| `-e, --config-overrides <KEY=VALUE>` | e.g. `networks.esmeralda.wallet-daemon-url=http://localhost:5100/json_rpc`. |
| `--skill` | Print this document. |

### `tari init [PATH]`

Sets up an existing crate: writes `tari.config.toml`, adds `tari_ootle_template_build` to
`[build-dependencies]`, creates `build.rs`, and fills `[package.metadata.tari-template]`.

Non-interactive form — pass everything, otherwise it prompts:

```bash
tari init -y --description "A fungible token" --tags token,defi --category token \
  --documentation https://docs.example.com --homepage https://example.com \
  --logo-url https://example.com/logo.png
```

### `tari create [NAME]` (alias `new`)

Scaffolds a new template crate from a starter template.

```bash
tari create my_token --template fungible -o ./crates
```

`--template` and `NAME` are prompted if omitted, so always pass both. Other options:
`--skip-init` (no `git init`), `--skip-metadata` (no `build.rs`/metadata setup), `-v/--verbose`.
The name is converted to `snake_case`.

### `tari build [PATH]`

Builds the WASM binary with the size-optimizing release profile and reports the binary and
metadata paths. `--no-cargo-opts` builds without the profile overrides.

### `tari lint [PATH]` (aliases `lints`, `check`)

Checks the crate for issues that bloat or break the published template.

| Option | Description |
|--------|-------------|
| `--fix` | Apply every fix that can be applied automatically |
| `--no-clippy` | Skip the `cargo clippy` run (manifest checks only) |
| `-D, --deny-warnings` | Exit non-zero on warnings and suggestions too |

Checks: `rust::clippy`, `cargo::crate-type` (must be exactly `["cdylib"]`),
`cargo::release-profile`, `cargo::test-runtime-profile`, `template::metadata`,
`template::metadata-build`, and `template::byte-vec`.

`template::byte-vec` has no autofix: `Vec<u8>` in CBOR-encoded template types and in template
function arguments/returns encodes as an array of integers. Replace it with `Bytes` from
`tari_template_lib::prelude`, or, for a struct field only, annotate it with
`#[cbor(with = "minicbor::bytes")]` (or `#[serde(with = "tari_template_lib::types::bytes")]`
for a serde-derived type).

Exit code `0` when no errors, `1` on any error (or any warning with `--deny-warnings`) — use
`tari lint --deny-warnings --no-clippy` in CI.

### `tari publish [PATH]` (alias `deploy`)

Builds (unless `--binary` is given) and publishes the template on-chain. **Costs fees.**

| Option | Description |
|--------|-------------|
| `-a, --account <NAME>` | Account paying the fees (default: config `default-account`, else the wallet default) |
| `-y, --yes` | Skip the confirmation prompt — **required for non-interactive runs** |
| `-f, --max-fee <MICROTARI>` | Fee cap (auto-estimated by default) |
| `--binary <PATH>` | Publish a pre-built WASM instead of building |
| `--wallet-daemon-url <URL>` | Overrides the active network's configured URL |
| `--publish-metadata` | Also submit metadata to the metadata server afterwards |
| `--metadata-server-url <URL>` | Metadata server for `--publish-metadata` |

The CLI aborts if the wallet daemon is on a different network than the active one. On success the
template address is saved to `[networks.<active>].template-address` in `tari.config.toml`, so
later `metadata publish` calls can omit `--template-address`.

### `tari template <init|inspect|publish>`

- `template init [PATH]` — metadata setup only (same options as `tari init`).
- `template inspect [PATH]` — print the built metadata; `--json` for machine-readable output,
  `--project-dir <PATH>` to search a different crate. Prefer `--json` when parsing.
- `template publish [PATH]` — same as `tari publish`.

### `tari metadata <publish|inspect>`

- `metadata inspect` — alias of `template inspect`.
- `metadata publish [PATH]` — submit metadata to a community metadata server.

| Option | Description |
|--------|-------------|
| `-t, --template-address <ADDR>` | Defaults to the address saved by `tari publish` |
| `--metadata-server-url <URL>` | Metadata server URL |
| `--signed` | Author-signed submission via the wallet daemon (lets metadata be updated without republishing on-chain) |
| `--key-index <N>` | Derived account key index with `--signed` (default `0`) |
| `--max-retries <N>` | Retries while the server has not synced the template yet (default `6`) |

Default (hash-verified) mode requires the template to have been published with a metadata hash.

### `tari config <init|set|get|show>`

Manages the project `tari.config.toml`.

```bash
tari config init
tari config set default-network localnet
tari config set networks.localnet.wallet-daemon-url http://localhost:12008/json_rpc
tari config get default-account
tari config show
```

## Configuration resolution

Active network: `--network` → project `default-network` → global `default-network` → `esmeralda`.

Per setting, highest priority first: CLI flag → project `tari.config.toml` → global config →
default. The wallet daemon URL defaults to `http://127.0.0.1:5100/json_rpc`. The API key is only
ever read from `--api-key` or `TARI_WALLET_DAEMON_API_KEY`.

## Wallet daemon authentication

Requests carry the API key as an `Authorization: Bearer` token; there is no interactive login.
The key must be minted with at least `templates:read`, `templates:create`, `accounts:read` and
`transactions:read`. If a command fails with an unauthorized error, the key is missing, expired,
or lacks a permission — do not retry blindly, tell the user which permission set is needed.

```bash
export TARI_WALLET_DAEMON_API_KEY="<key>"
tari publish -a myaccount -y
```

## Troubleshooting

| Symptom | Cause / fix |
|---------|-------------|
| Command hangs with no output | A prompt is waiting. Re-run with `-y`/`--non-interactive` and pass the missing values. |
| `wasm32-unknown-unknown` target errors | `rustup target add wasm32-unknown-unknown` |
| Network mismatch on publish | The wallet daemon runs a different network than `--network`/`default-network`. |
| Unauthorized from the wallet daemon | Missing or insufficient `TARI_WALLET_DAEMON_API_KEY`. |
| `metadata publish` 404s | The network has not synced the template yet; it retries with backoff, raise `--max-retries`. |
| Published binary is large | Run `tari lint --fix` — `crate-type` and the release profile are the usual causes. |
