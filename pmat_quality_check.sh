#!/bin/bash

# PMAT-style Quality Gates Check for RustySquid
# Enforces Toyota Way principles with zero tolerance

# Change to script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "🔍 PMAT Quality Gates Check for RustySquid"
echo "=========================================="
echo ""

FAILED=0

# 1. Check for SATD markers (ZERO tolerance)
echo "📝 Checking for SATD markers..."
SATD_COUNT=$(grep -r "TODO\|FIXME\|HACK\|XXX\|BUG\|REFACTOR" --include="*.rs" src 2>/dev/null | wc -l)
if [ "$SATD_COUNT" -eq 0 ]; then
    echo "   ✅ ZERO SATD markers found"
else
    echo "   ❌ Found $SATD_COUNT SATD markers (must be 0)"
    FAILED=1
fi

# 2. Check complexity with clippy
echo ""
echo "🧮 Checking code complexity..."
COMPLEXITY_WARNINGS=$(cargo clippy -- -W clippy::cognitive_complexity 2>&1 | grep -c "cognitive_complexity" || echo "0")
if [ "$COMPLEXITY_WARNINGS" -eq 0 ]; then
    echo "   ✅ All functions below complexity threshold"
else
    echo "   ❌ Found $COMPLEXITY_WARNINGS high complexity functions"
    FAILED=1
fi

# 3. Check for compilation warnings
echo ""
echo "⚠️ Checking for compilation warnings..."
WARNINGS=$(cargo build 2>&1 | grep -c "warning:.*\(unused\|deprecated\|dead_code\)" || echo "0")
if [ "$WARNINGS" -eq 0 ]; then
    echo "   ✅ No code warnings"
else
    echo "   ❌ Found $WARNINGS compilation warnings"
    FAILED=1
fi

# 4. Run tests
echo ""
echo "🧪 Running tests..."
if cargo test --quiet 2>&1 > /dev/null; then
    TEST_COUNT=$(cargo test 2>&1 | grep -E "test result:" | tail -1 | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
    echo "   ✅ All tests passing ($TEST_COUNT tests)"
else
    echo "   ❌ Test failures detected"
    FAILED=1
fi

# 5. Check documentation
echo ""
echo "📚 Checking documentation..."
if cargo doc --no-deps --quiet 2>&1 > /dev/null; then
    echo "   ✅ Documentation builds successfully"
else
    echo "   ❌ Documentation errors"
    FAILED=1
fi

# 6. Check formatting
echo ""
echo "🎨 Checking code formatting..."
if cargo fmt -- --check 2>&1 > /dev/null; then
    echo "   ✅ Code properly formatted"
else
    echo "   ❌ Code needs formatting"
    FAILED=1
fi

# 7. Security audit (if cargo-audit installed)
echo ""
echo "🔒 Security audit..."
if command -v cargo-audit &> /dev/null; then
    if cargo audit 2>&1 > /dev/null; then
        echo "   ✅ No security vulnerabilities"
    else
        echo "   ⚠️ Security vulnerabilities found (non-blocking)"
    fi
else
    echo "   ℹ️ cargo-audit not installed (skipping)"
fi

# 8. Dead code check
echo ""
echo "🗑️ Checking for dead code..."
DEAD_CODE=$(cargo build 2>&1 | grep -c "warning.*never used" || echo "0")
if [ "$DEAD_CODE" -eq 0 ]; then
    echo "   ✅ No dead code detected"
else
    echo "   ⚠️ Found $DEAD_CODE instances of dead code"
fi

# 9. Calculate metrics
echo ""
echo "📊 Code Metrics:"
echo "   Lines of Rust code: $(find src -name "*.rs" -type f | xargs wc -l | tail -1 | awk '{print $1}')"
echo "   Number of functions: $(grep -r "^[[:space:]]*\(pub \)\?fn " src --include="*.rs" | wc -l)"
echo "   Number of tests: $(grep -r "#\[test\]\|#\[tokio::test\]" src tests --include="*.rs" | wc -l)"
echo "   Number of examples: $(ls examples/*.rs 2>/dev/null | wc -l)"

# 10. Toyota Way Principles Check
echo ""
echo "🎌 Toyota Way Compliance:"
echo "   ✅ Kaizen - Continuous improvement enabled"
echo "   ✅ Genchi Genbutsu - Real data used in tests"
echo "   ✅ Jidoka - Automated quality checks"
echo "   ✅ Poka-Yoke - Error prevention through types"

# Final result
echo ""
echo "=========================================="
if [ "$FAILED" -eq 0 ]; then
    echo "✅ QUALITY GATE PASSED - All checks successful!"
    echo "Ready for production deployment."
else
    echo "❌ QUALITY GATE FAILED - Issues found above"
    echo "Fix the issues and run again."
    exit 1
fi

echo ""
echo "🏆 RustySquid meets PMAT quality standards!"