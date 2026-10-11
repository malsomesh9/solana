#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
anchor idl build -p solana_guard -o idl/solana_guard.json
cp idl/solana_guard.json app/src/idl/solana_guard.json
mkdir -p sdk/src/idl
cp idl/solana_guard.json sdk/src/idl/solana_guard.json
