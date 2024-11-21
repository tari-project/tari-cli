#!/usr/bin/env bash

# Copyright 2024 The Tari Project
# SPDX-License-Identifier: BSD-3-Clause

set -e

HASHES_PATH=meta/hashes.txt
SIG_OUTPUT_PATH=meta/hashes.txt.sig

gpg --output $SIG_OUTPUT_PATH --detach-sig $HASHES_PATH
