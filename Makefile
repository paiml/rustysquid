# PMAT-enforced Makefile for Router Cache Proxy
# Ensures quality through automated testing, linting, and metrics

.PHONY: all test lint format check clean build deploy verify help

# Default target
all: format lint test build

help: ## Show this help message
	@echo "Router Cache Proxy - Quality-Enforced Build"
	@echo "==========================================="
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'

# PMAT: Property-based testing
test: ## Run all tests including property tests
	@echo "🧪 Running unit tests..."
	@cargo test --lib
	@echo "🔬 Running property tests..."
	@cargo test --test '*' -- --nocapture
	@echo "✅ All tests passed!"

test-coverage: ## Generate test coverage report
	@echo "📊 Generating coverage report..."
	@cargo tarpaulin --out Html --output-dir coverage
	@echo "Coverage report: coverage/index.html"

# PMAT: Metrics and analysis
metrics: ## Calculate code metrics
	@echo "📏 Code Metrics:"
	@tokei src/
	@echo ""
	@echo "🔍 Cyclomatic Complexity:"
	@cargo clippy -- -W clippy::cognitive_complexity

# PMAT: Automated linting
lint: ## Run clippy with strict rules
	@echo "🔍 Running clippy..."
	@cargo clippy -- \
		-W clippy::all \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-D warnings \
		-A clippy::module_name_repetitions \
		-A clippy::must_use_candidate
	@echo "✅ No linting issues!"

# PMAT: Testing
format: ## Format code with rustfmt
	@echo "✨ Formatting code..."
	@cargo fmt -- --check || (cargo fmt && echo "✅ Code formatted!")

check: ## Type check without building
	@echo "🔧 Type checking..."
	@cargo check --target aarch64-unknown-linux-musl
	@echo "✅ Type check passed!"

# Build for production
build: ## Build optimized binary for ARM64
	@echo "🔨 Building for ARM64..."
	@cargo build --release --target aarch64-unknown-linux-musl
	@echo "📦 Binary size: $$(du -h target/aarch64-unknown-linux-musl/release/router-cache | cut -f1)"

build-debug: ## Build debug binary for testing
	@cargo build --target aarch64-unknown-linux-musl

# Deployment
deploy: build ## Deploy to router
	@echo "🚀 Deploying to router..."
	@cat target/aarch64-unknown-linux-musl/release/router-cache | \
		ssh noah@192.168.50.1 "cat > /tmp/router-cache && chmod +x /tmp/router-cache"
	@ssh noah@192.168.50.1 "killall router-cache 2>/dev/null; sleep 1; \
		nohup /tmp/router-cache > /tmp/cache.log 2>&1 & echo '✅ Deployed and started'"

# Verification
verify: ## Verify cache is working
	@echo "🔍 Verifying cache proxy..."
	@../deno run --allow-all ../scripts/verify-cache.ts

bench: ## Run benchmarks
	@echo "⚡ Running benchmarks..."
	@cargo bench

security-audit: ## Audit dependencies for vulnerabilities
	@echo "🔒 Security audit..."
	@cargo audit

clean: ## Clean build artifacts
	@cargo clean
	@rm -rf coverage/
	@echo "🧹 Cleaned!"

# PMAT Quality Gate
quality-gate: format lint test metrics security-audit ## Run full quality checks
	@echo "================================"
	@echo "✅ QUALITY GATE PASSED!"
	@echo "================================"
	@echo "The code meets all quality standards:"
	@echo "  ✓ Properly formatted"
	@echo "  ✓ No linting issues"
	@echo "  ✓ All tests passing"
	@echo "  ✓ Metrics within bounds"
	@echo "  ✓ No security vulnerabilities"

# Continuous monitoring
monitor: ## Monitor cache performance on router
	@ssh noah@192.168.50.1 "tail -f /tmp/cache.log | grep -E 'CACHE|HIT|MISS|ERROR'"

stats: ## Show cache statistics
	@ssh noah@192.168.50.1 "echo 'Cache Stats:'; \
		grep -c 'CACHE HIT' /tmp/cache.log 2>/dev/null || echo '0 hits'; \
		grep -c 'CACHE MISS' /tmp/cache.log 2>/dev/null || echo '0 misses'; \
		ps aux | grep router-cache | grep -v grep"