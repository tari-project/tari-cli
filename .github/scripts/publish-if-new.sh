#!/usr/bin/env bash
# Copyright 2026 The Tari Project
# SPDX-License-Identifier: BSD-3-Clause

# Publish a workspace crate to crates.io unless that exact name+version is
# already published. Lets the workflow be re-run safely after a partial
# failure (e.g. lib step succeeded, cli step failed).
set -euo pipefail

name=${1:?crate name required}
version=$(cargo metadata --format-version 1 --no-deps \
  | jq -r --arg name "$name" '.packages[] | select(.name == $name) | .version')

if [ -z "$version" ] || [ "$version" = "null" ]; then
  echo "error: could not resolve version for crate '$name'" >&2
  exit 1
fi

status=$(curl -fsS -o /dev/null -w "%{http_code}" \
  -H "User-Agent: github.com/tari-project/tari-cli publish workflow" \
  "https://crates.io/api/v1/crates/$name/$version" || echo "000")

case "$status" in
  200)
    echo "$name $version already on crates.io, skipping"
    ;;
  404)
    echo "$name $version not yet published, publishing"
    cargo publish --locked -p "$name"
    ;;
  *)
    echo "error: unexpected HTTP $status checking crates.io for $name $version" >&2
    exit 1
    ;;
esac
