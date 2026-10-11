#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo build-sbf --manifest-path programs/solana_guard/Cargo.toml -- --locked
cargo test --locked
# Anchor 1.0.2 dispatch macros trigger this Rust 1.94 Clippy lint.
cargo clippy --locked --all-targets -- -D warnings -A clippy::diverging_sub_expression
cargo run --locked --quiet --example sdk_vectors > sdk/test/abi-vectors.json
./scripts/sync-idl.sh
npm --prefix sdk ci --ignore-scripts
npm --prefix sdk test
npm --prefix app ci --ignore-scripts
npm --prefix app run build
python3 scripts/audit-scope.py --check
# Generated ABI and IDL must remain identical to reviewed source artifacts.
git diff --exit-code -- idl/solana_guard.json sdk/src/idl/solana_guard.json app/src/idl/solana_guard.json sdk/test/abi-vectors.json
