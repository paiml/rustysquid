#!/bin/bash
set -e

echo "=== RustySquid Crates.io Publishing Script ==="
echo

# Check if logged in to crates.io
if ! cargo login --help >/dev/null 2>&1; then
    echo "Please login to crates.io first:"
    echo "  cargo login"
    exit 1
fi

# Run tests
echo "1. Running tests..."
cargo test --all-features

# Check for warnings
echo
echo "2. Checking for warnings..."
cargo clippy -- -D warnings

# Check documentation
echo
echo "3. Building documentation..."
cargo doc --no-deps

# Dry run
echo
echo "4. Performing dry run..."
cargo publish --dry-run

# Confirm publication
echo
echo "=== Ready to publish RustySquid v1.0.0 to crates.io ==="
echo
echo "This will make the crate publicly available on crates.io"
echo "Repository: https://github.com/paiml/rustysquid"
echo
read -p "Do you want to proceed with publication? (y/N) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Publishing to crates.io..."
    cargo publish
    echo
    echo "✅ Successfully published!"
    echo "View at: https://crates.io/crates/rustysquid"
else
    echo "Publication cancelled."
fi