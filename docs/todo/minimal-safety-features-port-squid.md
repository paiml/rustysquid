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

## References

- [Squid Cache Source](https://github.com/squid-cache/squid)
- [HTTP Caching RFC 7234](https://tools.ietf.org/html/rfc7234)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Tokio Tracing](https://tokio.rs/tokio/topics/tracing)

## Notes

- Each task should be a separate PR
- Run benchmarks before/after each phase
- Deploy to test router between phases
- Document performance impact of each feature
- Keep total binary size < 1MB