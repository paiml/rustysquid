# Minimal Safety Features: Port from Squid Cache

## Overview
Port the minimal subset of Squid cache safety features to fix critical defects in our Rust cache proxy while maintaining low memory footprint for router deployment.

## Critical Issues to Fix
1. **Memory exhaustion** - No total cache size limit
2. **Crash on malformed input** - Multiple unwrap() calls
3. **No request limits** - Can consume unlimited memory
4. **Blocking I/O in async** - println! blocks runtime
5. **Silent failures** - Errors ignored with `let _`

## Phase 1: Memory Safety [CRITICAL]

### Task 1.1: Implement Total Cache Size Limit
- [ ] Add `total_size: AtomicUsize` to track total cached bytes
- [ ] Add `MAX_CACHE_BYTES: usize = 50 * 1024 * 1024` (50MB limit)
- [ ] Implement LRU eviction when size exceeded
- [ ] Add size tracking on insert/remove
- **Tests Required:**
  - [ ] Unit: Test size tracking accuracy
  - [ ] Property: Cache never exceeds MAX_CACHE_BYTES
  - [ ] Doc: Example showing eviction behavior
- **Verification:** `cargo test test_cache_size_limit`

### Task 1.2: Add Per-Entry Size Limit
- [ ] Add `MAX_ENTRY_SIZE: usize = 5 * 1024 * 1024` (5MB per entry)
- [ ] Reject responses larger than limit
- [ ] Add configuration option for size
- **Tests Required:**
  - [ ] Unit: Test rejection of oversized entries
  - [ ] Property: No entry exceeds MAX_ENTRY_SIZE
  - [ ] Doc: Example of size rejection
- **Verification:** `cargo test test_entry_size_limit`

### Task 1.3: Implement Memory Pressure Detection
- [ ] Add memory usage check before caching
- [ ] Skip caching when system memory low
- [ ] Add `/proc/meminfo` parsing for Linux
- **Tests Required:**
  - [ ] Unit: Mock memory pressure scenarios
  - [ ] Property: Cache operations safe under memory pressure
  - [ ] Doc: Memory pressure handling example
- **Verification:** `cargo test test_memory_pressure`

## Phase 2: Request Safety [HIGH]

### Task 2.1: Add Request Size Limits
- [ ] Add `MAX_REQUEST_SIZE: usize = 64 * 1024` (64KB headers)
- [ ] Implement streaming request parser
- [ ] Reject oversized requests with 413 error
- **Tests Required:**
  - [ ] Unit: Test request size enforcement
  - [ ] Property: Parser never allocates > MAX_REQUEST_SIZE
  - [ ] Doc: Example of handling large requests
- **Verification:** `cargo test test_request_limits`

### Task 2.2: Implement Request Timeout
- [ ] Add per-request timeout (30s default)
- [ ] Cancel upstream connection on timeout
- [ ] Return 504 Gateway Timeout
- **Tests Required:**
  - [ ] Unit: Test timeout behavior
  - [ ] Property: All requests complete or timeout
  - [ ] Doc: Timeout configuration example
- **Verification:** `cargo test test_request_timeout`

### Task 2.3: Add Connection Limits
- [ ] Add `MAX_CONNECTIONS: usize = 100`
- [ ] Implement connection counting
- [ ] Reject new connections when limit reached
- **Tests Required:**
  - [ ] Unit: Test connection limiting
  - [ ] Property: Never exceed MAX_CONNECTIONS
  - [ ] Doc: Connection limit behavior
- **Verification:** `cargo test test_connection_limits`

## Phase 3: Error Handling [HIGH]

### Task 3.1: Replace All unwrap() Calls
- [ ] Replace `SystemTime::now().duration_since().unwrap()` with safe alternative
- [ ] Replace `NonZeroUsize::new().unwrap()` with expect() and context
- [ ] Replace `parse::<u16>().unwrap_or()` with proper error handling
- [ ] Add custom error types
- **Tests Required:**
  - [ ] Unit: Test error paths
  - [ ] Property: No panics on any input
  - [ ] Doc: Error handling examples
- **Verification:** `cargo test --no-fail-fast && grep -c "unwrap()" src/*.rs | grep "^0$"`

### Task 3.2: Implement Result-based Error Propagation
- [ ] Change handle_client to return Result<(), ProxyError>
- [ ] Add ProxyError enum with variants
- [ ] Propagate errors with ? operator
- **Tests Required:**
  - [ ] Unit: Test each error variant
  - [ ] Property: All errors handled gracefully
  - [ ] Doc: Error type documentation
- **Verification:** `cargo test test_error_propagation`

### Task 3.3: Add Error Recovery
- [ ] Implement exponential backoff for upstream failures
- [ ] Add circuit breaker for failing upstreams
- [ ] Cache negative responses (404s) briefly
- **Tests Required:**
  - [ ] Unit: Test retry logic
  - [ ] Property: Backoff increases exponentially
  - [ ] Doc: Recovery strategy examples
- **Verification:** `cargo test test_error_recovery`

## Phase 4: Logging & Monitoring [MEDIUM]

### Task 4.1: Replace println! with Async Logger
- [ ] Add `tracing` crate for structured logging
- [ ] Replace all println! with tracing macros
- [ ] Add log levels (ERROR, WARN, INFO, DEBUG)
- [ ] Implement rotating file appender
- **Tests Required:**
  - [ ] Unit: Test log output formats
  - [ ] Property: Logging never blocks
  - [ ] Doc: Logging configuration example
- **Verification:** `cargo test test_async_logging && grep -c "println!" src/*.rs | grep "^0$"`

### Task 4.2: Add Metrics Collection
- [ ] Track cache hit/miss ratio
- [ ] Track response times (p50, p95, p99)
- [ ] Track memory usage
- [ ] Add Prometheus-compatible metrics endpoint
- **Tests Required:**
  - [ ] Unit: Test metric collection
  - [ ] Property: Metrics accurate under load
  - [ ] Doc: Metrics endpoint usage
- **Verification:** `cargo test test_metrics`

### Task 4.3: Implement Health Check Endpoint
- [ ] Add `/health` endpoint on separate port
- [ ] Return cache stats in JSON
- [ ] Include memory usage and uptime
- **Tests Required:**
  - [ ] Unit: Test health endpoint
  - [ ] Property: Health check always responds
  - [ ] Doc: Health check integration
- **Verification:** `cargo test test_health_check`

## Phase 5: Cache Intelligence [MEDIUM]

### Task 5.1: Port Squid's Cache-Control Parser
- [ ] Parse Cache-Control directives properly
- [ ] Respect no-cache, no-store, private
- [ ] Handle max-age, s-maxage correctly
- [ ] Implement must-revalidate
- **Tests Required:**
  - [ ] Unit: Test each directive
  - [ ] Property: Cache-Control always respected
  - [ ] Doc: Cache-Control examples
- **Verification:** `cargo test test_cache_control`

### Task 5.2: Implement Vary Header Support
- [ ] Store Vary headers with cached responses
- [ ] Match requests based on Vary headers
- [ ] Support multiple cached variants
- **Tests Required:**
  - [ ] Unit: Test Vary matching
  - [ ] Property: Correct variant always returned
  - [ ] Doc: Vary header examples
- **Verification:** `cargo test test_vary_header`

### Task 5.3: Add Conditional Request Support
- [ ] Support If-None-Match (ETag)
- [ ] Support If-Modified-Since
- [ ] Return 304 Not Modified when appropriate
- **Tests Required:**
  - [ ] Unit: Test conditional requests
  - [ ] Property: 304s returned correctly
  - [ ] Doc: Conditional request flow
- **Verification:** `cargo test test_conditional_requests`

## Phase 6: Security Features [LOW-MEDIUM]

### Task 6.1: Add Basic ACL Support
- [ ] Implement IP-based allow/deny lists
- [ ] Add configurable ACL rules
- [ ] Block requests based on patterns
- **Tests Required:**
  - [ ] Unit: Test ACL matching
  - [ ] Property: ACLs correctly enforced
  - [ ] Doc: ACL configuration
- **Verification:** `cargo test test_acl`

### Task 6.2: Implement Request Sanitization
- [ ] Strip dangerous headers (X-Forwarded-For manipulation)
- [ ] Validate Host header
- [ ] Prevent request smuggling
- **Tests Required:**
  - [ ] Unit: Test header sanitization
  - [ ] Property: No malicious headers pass through
  - [ ] Doc: Security features
- **Verification:** `cargo test test_request_sanitization`

### Task 6.3: Add Rate Limiting
- [ ] Implement token bucket rate limiter
- [ ] Per-IP rate limits
- [ ] Return 429 Too Many Requests
- **Tests Required:**
  - [ ] Unit: Test rate limiting
  - [ ] Property: Rate limits enforced accurately
  - [ ] Doc: Rate limit configuration
- **Verification:** `cargo test test_rate_limiting`

## Implementation Order (by Priority)

1. **Week 1: Memory Safety** (Tasks 1.1-1.3) - Prevents OOM crashes
2. **Week 2: Error Handling** (Tasks 3.1-3.3) - Prevents panics
3. **Week 3: Request Safety** (Tasks 2.1-2.3) - Prevents DoS
4. **Week 4: Logging** (Tasks 4.1-4.3) - Enables debugging
5. **Week 5: Cache Intelligence** (Tasks 5.1-5.3) - Improves correctness
6. **Week 6: Security** (Tasks 6.1-6.3) - Hardens deployment

## Test Requirements

### For Each Task:
1. **Unit Tests**: Test the specific functionality
   ```rust
   #[test]
   fn test_feature() { /* ... */ }
   ```

2. **Property Tests**: Test invariants hold
   ```rust
   proptest! {
       #[test]
       fn prop_feature_invariant(input in any::<Input>()) {
           // Property assertion
       }
   }
   ```

3. **Doc Tests**: Provide usage examples
   ```rust
   /// # Example
   /// ```
   /// use crate::feature;
   /// let result = feature::use_case();
   /// assert!(result.is_ok());
   /// ```
   ```

## Verification Commands

```bash
# Run all tests for a phase
make test-phase PHASE=1

# Check specific task completion
make verify-task TASK=1.1

# Run property tests only
cargo test --test property_tests

# Check for unsafe patterns
make safety-check

# Benchmark before/after
make bench-compare
```

## Success Criteria

- [ ] No panics under any input (fuzz tested)
- [ ] Memory usage < 100MB under load
- [ ] 99.9% uptime over 7 days
- [ ] < 10ms added latency for cache hits
- [ ] All tests passing (unit, property, doc)
- [ ] No `unwrap()` calls in production code
- [ ] Structured logging with rotation
- [ ] Prometheus metrics exposed

## Phase 7: Quality Standards & Release [HIGH PRIORITY]

### Task 7.1: Adopt paiml-mcp-agent-toolkit Quality Standards
- [ ] Implement continuous integration with GitHub Actions
- [ ] Add semantic versioning (0.1.0-alpha -> 0.1.0-beta -> 0.1.0)
- [ ] Create CHANGELOG.md with Keep a Changelog format
- [ ] Add code coverage badges and maintain > 80% coverage
- [ ] Implement dependency security scanning
- [ ] Add CONTRIBUTING.md with clear guidelines
- **Quality Standards from paiml-mcp-agent-toolkit:**
  - [ ] Automated release workflow on tag push
  - [ ] Cross-platform testing matrix (Linux, macOS, Windows)
  - [ ] Dependency audit in CI pipeline
  - [ ] Automated benchmarking with performance regression detection
  - [ ] Documentation generation and deployment
- **Tests Required:**
  - [ ] CI/CD pipeline validation
  - [ ] Release artifact verification
  - [ ] Cross-platform compatibility tests
- **Verification:** `gh workflow run ci.yml && cargo audit`

### Task 7.2: Binary Size Optimization
- [ ] Enable LTO (Link Time Optimization) in release profile
- [ ] Strip debug symbols with `strip = true`
- [ ] Use `opt-level = "z"` for size optimization
- [ ] Replace heavy dependencies with lighter alternatives:
  - [ ] Consider `smol` instead of `tokio` for smaller runtime
  - [ ] Use `minihttpse` for HTTP parsing if smaller
  - [ ] Evaluate `tinyvec` instead of `Vec` for fixed-size collections
- [ ] Enable `panic = "abort"` to reduce binary size
- [ ] Use `codegen-units = 1` for better optimization
- [ ] Profile with `cargo bloat` to identify large functions
- [ ] Consider `#[no_std]` for core components
- **Target Metrics:**
  - [ ] Binary size < 500KB stripped (currently 520KB)
  - [ ] Memory usage < 5MB idle (currently ~10MB)
  - [ ] Startup time < 100ms
- **Tests Required:**
  - [ ] Size regression tests in CI
  - [ ] Memory profiling under load
  - [ ] Startup performance benchmarks
- **Verification:** `cargo bloat --release && strip target/release/rustysquid && ls -lh`

### Task 7.3: Crates.io Release Preparation
- [ ] Ensure all metadata in Cargo.toml is complete:
  - [ ] Add `documentation` field pointing to docs.rs
  - [ ] Add `homepage` field
  - [ ] Verify `keywords` are relevant (max 5)
  - [ ] Ensure `categories` are valid crates.io categories
  - [ ] Add comprehensive `description`
- [ ] Create examples/ directory with usage examples
- [ ] Add integration tests in tests/ directory
- [ ] Generate and review API documentation
- [ ] Add README badges:
  - [ ] Crates.io version
  - [ ] Documentation
  - [ ] License
  - [ ] Build status
  - [ ] Coverage
- [ ] Set up docs.rs documentation build
- **Release Checklist:**
  - [ ] All tests passing
  - [ ] No security advisories from `cargo audit`
  - [ ] Documentation complete
  - [ ] CHANGELOG updated
  - [ ] Version bumped
- **Verification:** `cargo publish --dry-run && cargo doc --open`

### Task 7.4: GitHub Binary Releases
- [ ] Create GitHub Actions workflow for releases:
  ```yaml
  name: Release
  on:
    push:
      tags:
        - 'v*'
  ```
- [ ] Build matrix for multiple targets:
  - [ ] x86_64-unknown-linux-musl (static Linux)
  - [ ] aarch64-unknown-linux-musl (ARM64 routers)
  - [ ] armv7-unknown-linux-musleabihf (ARM32)
  - [ ] x86_64-apple-darwin (macOS)
  - [ ] x86_64-pc-windows-msvc (Windows)
- [ ] Generate SHA256 checksums for all binaries
- [ ] Create release notes from CHANGELOG
- [ ] Upload binaries as release artifacts
- [ ] Add installation script for easy deployment
- **Binary Naming Convention:**
  - `rustysquid-v0.1.0-x86_64-linux`
  - `rustysquid-v0.1.0-aarch64-linux`
  - `rustysquid-v0.1.0-armv7-linux`
- **Tests Required:**
  - [ ] Release workflow validation
  - [ ] Binary verification on target platforms
  - [ ] Installation script testing
- **Verification:** `gh release create v0.1.0-rc1 --prerelease`

## Phase 8: Official 0.1.0 Release

### Release Criteria for 0.1.0:
- [ ] All Phase 1-3 tasks complete (Memory, Request, Error safety)
- [ ] Zero panics in 24-hour fuzz testing
- [ ] Binary size < 500KB for ARM64
- [ ] Memory usage < 10MB under normal load
- [ ] 100% test coverage for safety-critical paths
- [ ] Documentation complete on docs.rs
- [ ] CI/CD pipeline fully operational

### Release Process:
1. **Beta Testing** (2 weeks)
   - Deploy to test routers
   - Collect performance metrics
   - Fix any discovered issues

2. **Release Candidate**
   ```bash
   cargo version 0.1.0-rc1
   git tag v0.1.0-rc1
   git push origin v0.1.0-rc1
   ```

3. **Final Release**
   ```bash
   cargo version 0.1.0
   cargo publish
   git tag v0.1.0
   git push origin v0.1.0
   gh release create v0.1.0 --title "RustySquid 0.1.0" --notes-file CHANGELOG.md
   ```

### Post-Release:
- [ ] Announce on Reddit r/rust, r/selfhosted
- [ ] Update router community forums
- [ ] Create Docker image for easy deployment
- [ ] Add to Awesome Rust list

## Quality Metrics Dashboard

### Code Quality
- **Coverage**: Target > 80% (Current: TBD)
- **Clippy Warnings**: 0 with pedantic lints
- **Documentation**: 100% public API documented
- **Unsafe Code**: 0 unsafe blocks
- **Dependencies**: < 20 total, all audited

### Performance Metrics
- **Binary Size**: < 500KB (Current: 520KB)
- **Memory Usage**: < 10MB idle, < 50MB under load
- **Cache Hit Time**: < 1ms p99
- **Cache Miss Overhead**: < 5ms p99
- **Throughput**: > 100 Mbps on router hardware

### Reliability Metrics
- **Uptime**: 99.9% over 30 days
- **Crash Rate**: 0 panics in production
- **Error Rate**: < 0.1% requests failed
- **Recovery Time**: < 1s after crash

## References

- [Squid Cache Source](https://github.com/squid-cache/squid)
- [HTTP Caching RFC 7234](https://tools.ietf.org/html/rfc7234)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Tokio Tracing](https://tokio.rs/tokio/topics/tracing)
- [paiml-mcp-agent-toolkit](https://github.com/paiml/mcp-agent-toolkit) - Quality standards reference
- [min-sized-rust](https://github.com/johnthagen/min-sized-rust) - Binary size optimization
- [cargo-release](https://github.com/crate-ci/cargo-release) - Release automation

## Notes

- Each task should be a separate PR
- Run benchmarks before/after each phase
- Deploy to test router between phases
- Document performance impact of each feature
- Keep total binary size < 500KB (optimized from 1MB target)
- Follow semver strictly: 0.1.0-alpha.1 -> 0.1.0-beta.1 -> 0.1.0-rc.1 -> 0.1.0