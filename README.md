# RustySquid 🦀🦑

[![Rust CI](https://github.com/paiml/rustysquid/actions/workflows/rust.yml/badge.svg)](https://github.com/paiml/rustysquid/actions/workflows/rust.yml)
[![CI](https://github.com/paiml/rustysquid/actions/workflows/ci.yml/badge.svg)](https://github.com/paiml/rustysquid/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rustysquid.svg)](https://crates.io/crates/rustysquid)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A minimal, memory-safe HTTP caching proxy inspired by Squid, designed for embedded systems and routers.

## Features

- ✅ **100% Safe Rust**: No unsafe code, no panics in production
- ✅ **Memory Limits**: Enforced cache size (50MB) and per-entry limits (5MB)
- ✅ **DoS Protection**: Connection limits (100), request size limits (64KB)
- ✅ **Memory Pressure Detection**: Skips caching when system memory is low
- ✅ **Graceful Shutdown**: Handles SIGTERM/SIGINT properly
- ✅ **Async Logging**: Non-blocking tracing instead of println!
- ✅ **LRU Eviction**: Automatic cache management when full
- **Fast**: Sub-millisecond cache hits, 4000x speedup on cached content
- **HTTP/1.1 Compliant**: Respects Cache-Control headers
- **Embedded-Friendly**: Single-threaded tokio runtime for low resource usage

## Current Status

✅ **v1.1.0 Production Ready** - PMAT quality standards integrated

## Examples

RustySquid includes comprehensive examples demonstrating various use cases:

### Simple Proxy Demo
```bash
cargo run --example simple_proxy
```
Demonstrates basic cache operations and proxy setup.

### Cache Operations Demo
```bash
cargo run --example cache_demo
```
Shows cacheability tests, TTL calculation, expiration handling, and size limits.

### Performance Testing
```bash
cargo run --example performance_test --release
```
Measures cache performance including sequential/concurrent operations and hit vs miss timing.

### Full Proxy Server
```bash
cargo run --example full_proxy
```
Runs a complete HTTP proxy server on port 8888 with statistics tracking.

## Quick Start

### Build for ARM64 Router
```bash
cargo build --release --target aarch64-unknown-linux-musl
```

### Deploy to Router
```bash
cat target/aarch64-unknown-linux-musl/release/rustysquid | \
  ssh user@router "cat > /tmp/rustysquid && chmod +x /tmp/rustysquid"
ssh user@router "nohup /tmp/rustysquid > /tmp/cache.log 2>&1 &"
```

### Configure Client
```bash
export http_proxy=http://router:3128
export https_proxy=http://router:3128
```

## Configuration

Currently uses compile-time constants:
- `CACHE_SIZE`: 10,000 entries
- `MAX_RESPONSE_SIZE`: 10MB
- `CACHE_TTL`: 3600 seconds
- `PROXY_PORT`: 3128

## Testing

```bash
# Run all tests
cargo test

# Property-based tests
cargo test --test property_tests

# Safety verification
make -f Makefile.safety safety-check

# Benchmarks
cargo bench
```

## Safety Implementation Plan

We're implementing minimal safety features from Squid in 6 phases:

1. **Memory Safety** - Prevent OOM crashes
2. **Request Safety** - Prevent DoS attacks
3. **Error Handling** - Remove panics
4. **Logging** - Async structured logging
5. **Cache Intelligence** - HTTP compliance
6. **Security** - ACLs and rate limiting

See [detailed plan](../docs/todo/minimal-safety-features-port-squid.md) for task breakdown.

## Performance

On ASUS RT-AX88U router (1GB RAM, Quad-core ARM):
- Cache hits: < 1ms latency
- Cache misses: Adds < 5ms overhead
- Memory usage: < 10MB for 1000 cached entries
- Hit rate: 40-60% for typical browsing

## Why RustySquid?

Unlike full Squid (200MB+ memory), RustySquid targets resource-constrained environments:
- Home routers
- Raspberry Pi
- Edge devices
- IoT gateways

## Contributing

We follow PMAT methodology:
- **P**roperty-based testing for invariants
- **M**etrics for performance tracking
- **A**utomated testing (unit, integration, doc)
- **T**esting before merging

Each PR should:
1. Add tests for new features
2. Pass `make safety-check`
3. Include benchmarks for performance changes
4. Update documentation

## License

MIT

## Acknowledgments

Inspired by:
- [Squid Cache](http://www.squid-cache.org/) - The legendary caching proxy
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- ASUS-WRT firmware community

---

**Note**: This is an educational project demonstrating safe Rust practices for network programming. Use in production at your own risk.