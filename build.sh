#!/usr/bin/env bash
set -e

export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

echo "Building totalrecall..."
cargo build "$@"

echo ""
echo "Build successful!"
echo "Run: cargo run --bin rag-mcp -- --help"
