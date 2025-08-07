#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║           RustySquid Complete Quality Check                   ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo

cd "$(dirname "$0")"

# Track failures
FAILED=0

# 1. Format check
echo "▶ Formatting Check"
if cargo fmt --check 2>/dev/null; then
    echo "  ✅ Code is properly formatted"
else
    echo "  🔧 Formatting code..."
    cargo fmt
    echo "  ✅ Code formatted"
fi
echo

# 2. Clippy check
echo "▶ Clippy Analysis"
if cargo clippy --all-targets --all-features -- \
    -W clippy::pedantic \
    -W clippy::nursery \
    -A clippy::module_name_repetitions \
    -A clippy::must_use_candidate \
    -A clippy::missing_errors_doc \
    -A clippy::missing_panics_doc \
    -D warnings 2>&1 | grep -q "warning"; then
    echo "  ⚠️  Clippy warnings found"
    FAILED=1
else
    echo "  ✅ No clippy warnings"
fi
echo

# 3. Test suite
echo "▶ Test Suite"
if cargo test --all-features --quiet; then
    echo "  ✅ All tests passed"
else
    echo "  ❌ Tests failed"
    FAILED=1
fi
echo

# 4. Documentation
echo "▶ Documentation"
if cargo doc --no-deps --quiet 2>/dev/null; then
    echo "  ✅ Documentation builds successfully"
else
    echo "  ❌ Documentation build failed"
    FAILED=1
fi
echo

# 5. Safety checks
echo "▶ Safety Checks"
UNSAFE_COUNT=$(grep -r "unsafe" src/ --include="*.rs" 2>/dev/null | grep -v "// " | wc -l)
UNWRAP_COUNT=$(grep -r "\.unwrap()" src/ --include="*.rs" | grep -v "test" | grep -v "#\[cfg(test)\]" | wc -l)
PRINTLN_COUNT=$(grep -r "println!" src/ --include="*.rs" | grep -v "test" | wc -l)

if [ "$UNSAFE_COUNT" -eq 0 ]; then
    echo "  ✅ No unsafe code"
else
    echo "  ❌ Found $UNSAFE_COUNT unsafe blocks"
    FAILED=1
fi

if [ "$UNWRAP_COUNT" -eq 0 ]; then
    echo "  ✅ No unwrap() in production"
else
    echo "  ⚠️  Found $UNWRAP_COUNT unwrap() calls"
fi

if [ "$PRINTLN_COUNT" -eq 0 ]; then
    echo "  ✅ No println! in production"
else
    echo "  ⚠️  Found $PRINTLN_COUNT println! calls"
fi
echo

# 6. Build check
echo "▶ Build Check"
if cargo build --release --quiet 2>/dev/null; then
    SIZE=$(ls -lh target/release/rustysquid 2>/dev/null | awk '{print $5}')
    echo "  ✅ Release build successful (size: $SIZE)"
else
    echo "  ❌ Build failed"
    FAILED=1
fi
echo

# 7. Dependency audit
echo "▶ Security Audit"
if command -v cargo-audit >/dev/null 2>&1; then
    if cargo audit 2>/dev/null | grep -q "vulnerabilities found"; then
        echo "  ❌ Security vulnerabilities found"
        FAILED=1
    else
        echo "  ✅ No known vulnerabilities"
    fi
else
    echo "  ⏭️  Skipping (cargo-audit not installed)"
fi
echo

# Summary
echo "════════════════════════════════════════════════════════════════"
if [ $FAILED -eq 0 ]; then
    echo "✅ ALL QUALITY CHECKS PASSED!"
    echo
    echo "The code is ready for:"
    echo "  • Production deployment"
    echo "  • Publishing to crates.io"
    echo "  • Router installation"
else
    echo "❌ SOME QUALITY CHECKS FAILED"
    echo
    echo "Please fix the issues above before proceeding."
fi
echo "════════════════════════════════════════════════════════════════"

exit $FAILED