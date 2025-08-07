#!/bin/bash

echo "=== Checking Dependencies ==="
echo

# Check for unused dependencies
echo "1. Checking for unused dependencies..."
cargo machete 2>/dev/null || {
    echo "   Installing cargo-machete..."
    cargo install cargo-machete
}
cargo machete

echo
echo "2. Checking dependency tree..."
cargo tree --duplicates

echo
echo "3. Checking for outdated dependencies..."
cargo outdated 2>/dev/null || {
    echo "   Installing cargo-outdated..."
    cargo install cargo-outdated
}
cargo outdated

echo
echo "4. Security audit..."
cargo audit 2>/dev/null || {
    echo "   Installing cargo-audit..."
    cargo install cargo-audit
}
cargo audit

echo
echo "=== Dependency check complete ==="