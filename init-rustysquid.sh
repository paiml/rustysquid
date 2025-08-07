#!/bin/bash
# Initialize RustySquid repository

echo "🦑 Initializing RustySquid repository..."

# Initialize git if not already
if [ ! -d .git ]; then
    git init
    echo "✅ Git repository initialized"
fi

# Add remote
git remote remove origin 2>/dev/null
git remote add origin git@github.com:paiml/rustysquid.git
echo "✅ Remote repository set to: git@github.com:paiml/rustysquid.git"

# Create .gitignore
cat > .gitignore << 'EOF'
# Rust
target/
Cargo.lock
*.rs.bk
*.pdb

# Coverage
coverage/
*.lcov
tarpaulin-report.html

# Benchmarks
bench-*.txt

# Editor
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
/tmp/

# Test artifacts
*.profraw
*.profdata
EOF
echo "✅ .gitignore created"

# Initial commit structure
echo "📁 Repository structure:"
echo "  src/           - Source code"
echo "  tests/         - Test files"
echo "  benches/       - Benchmarks (to be added)"
echo "  docs/todo/     - Implementation plans"
echo ""

echo "📋 Next steps:"
echo "  1. Review safety implementation plan: ../docs/todo/minimal-safety-features-port-squid.md"
echo "  2. Check current safety issues: make -f Makefile.safety safety-check"
echo "  3. Run tests: cargo test"
echo "  4. Build for router: cargo build --release --target aarch64-unknown-linux-musl"
echo ""

echo "🚀 To push to GitHub:"
echo "  git add ."
echo "  git commit -m 'Initial commit: RustySquid - Minimal HTTP cache proxy'"
echo "  git push -u origin main"
echo ""

echo "✅ RustySquid initialization complete!"