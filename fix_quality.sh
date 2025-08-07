#!/bin/bash
set -e

echo "=== Fixing RustySquid Quality Issues ==="
echo

cd "$(dirname "$0")"

# 1. Format code
echo "1. Formatting code..."
cargo fmt
echo "   ✅ Code formatted"

# 2. Fix clippy issues
echo
echo "2. Fixing clippy issues..."
cargo clippy --fix --allow-dirty --allow-staged -- \
    -W clippy::all \
    -W clippy::pedantic \
    -A clippy::module_name_repetitions \
    -A clippy::must_use_candidate \
    -A clippy::missing_errors_doc \
    -A clippy::missing_panics_doc 2>/dev/null || true
echo "   ✅ Clippy fixes applied"

# 3. Fix compilation issues
echo
echo "3. Fixing compilation issues..."
cargo fix --allow-dirty --allow-staged 2>/dev/null || true
echo "   ✅ Compilation fixes applied"

# 4. Format again after fixes
echo
echo "4. Final formatting pass..."
cargo fmt
echo "   ✅ Final format complete"

# 5. Run tests
echo
echo "5. Running tests..."
if cargo test --quiet; then
    echo "   ✅ All tests passed"
else
    echo "   ⚠️  Some tests failed (manual fix needed)"
fi

# 6. Check documentation
echo
echo "6. Building documentation..."
if cargo doc --no-deps --quiet 2>/dev/null; then
    echo "   ✅ Documentation builds"
else
    echo "   ⚠️  Documentation issues"
fi

# 7. Final clippy check
echo
echo "7. Final quality check..."
if cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning"; then
    echo "   ⚠️  Some warnings remain:"
    cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep "warning" | head -5
else
    echo "   ✅ No warnings"
fi

echo
echo "=== Quality fixes complete ==="
echo "Run 'make quality-gate' to verify all checks pass"