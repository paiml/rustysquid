#!/bin/bash
set -e

echo "=== RustySquid Quality Checks ==="
echo

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

FAILED=0

echo "1. Running cargo fmt..."
if cargo fmt --check; then
    echo -e "${GREEN}✅ Formatting correct${NC}"
else
    echo -e "${YELLOW}⚠️  Formatting issues found, fixing...${NC}"
    cargo fmt
    echo -e "${GREEN}✅ Formatting fixed${NC}"
fi

echo
echo "2. Running cargo clippy..."
if cargo clippy -- -D warnings 2>&1 | grep -q "warning"; then
    echo -e "${YELLOW}⚠️  Clippy warnings found${NC}"
    cargo clippy -- -D warnings
    FAILED=1
else
    cargo clippy -- -D warnings
    echo -e "${GREEN}✅ No clippy warnings${NC}"
fi

echo
echo "3. Checking for unsafe code..."
if grep -r "unsafe" src/ --include="*.rs" | grep -v "// unsafe" | grep -v "UNSAFE" | grep -v "test"; then
    echo -e "${RED}❌ Unsafe code found${NC}"
    FAILED=1
else
    echo -e "${GREEN}✅ No unsafe code${NC}"
fi

echo
echo "4. Checking for unwrap() in production..."
UNWRAPS=$(grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | grep -v "#\[cfg(test)\]" -A5 -B5 | grep "unwrap()" | wc -l)
if [ "$UNWRAPS" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Found $UNWRAPS unwrap() calls in production code${NC}"
    grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | head -5
else
    echo -e "${GREEN}✅ No unwrap() in production code${NC}"
fi

echo
echo "5. Checking for println! in production..."
PRINTS=$(grep -r "println!" src/ --include="*.rs" | grep -v "test" | wc -l)
if [ "$PRINTS" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Found $PRINTS println! calls${NC}"
    grep -r "println!" src/ --include="*.rs" | grep -v "test" | head -5
else
    echo -e "${GREEN}✅ No println! in production code${NC}"
fi

echo
echo "6. Running tests..."
if cargo test --quiet; then
    echo -e "${GREEN}✅ All tests passed${NC}"
else
    echo -e "${RED}❌ Tests failed${NC}"
    FAILED=1
fi

echo
echo "7. Building documentation..."
if cargo doc --no-deps --quiet; then
    echo -e "${GREEN}✅ Documentation builds${NC}"
else
    echo -e "${RED}❌ Documentation build failed${NC}"
    FAILED=1
fi

echo
echo "8. Checking dependencies..."
if cargo tree --duplicate 2>&1 | grep -q "package"; then
    echo -e "${YELLOW}⚠️  Duplicate dependencies found${NC}"
    cargo tree --duplicate | head -10
else
    echo -e "${GREEN}✅ No duplicate dependencies${NC}"
fi

echo
echo "9. Checking binary size..."
cargo build --release --target x86_64-unknown-linux-musl 2>/dev/null || cargo build --release
SIZE=$(ls -lh target/*/release/rustysquid | head -1 | awk '{print $5}')
echo "Binary size: $SIZE"
if [[ "$SIZE" == *"M"* ]]; then
    SIZE_NUM=$(echo $SIZE | sed 's/M//')
    if (( $(echo "$SIZE_NUM > 2" | bc -l) )); then
        echo -e "${YELLOW}⚠️  Binary size larger than target (2MB)${NC}"
    else
        echo -e "${GREEN}✅ Binary size acceptable${NC}"
    fi
else
    echo -e "${GREEN}✅ Binary size acceptable${NC}"
fi

echo
echo "10. License check..."
if [ -f "../LICENSE" ] || [ -f "LICENSE" ]; then
    echo -e "${GREEN}✅ LICENSE file present${NC}"
else
    echo -e "${RED}❌ LICENSE file missing${NC}"
    FAILED=1
fi

echo
echo "════════════════════════════════════════"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL QUALITY CHECKS PASSED!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some quality checks failed${NC}"
    exit 1
fi