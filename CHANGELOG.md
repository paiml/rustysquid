# Changelog

All notable changes to RustySquid will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial HTTP/1.1 proxy implementation
- LRU cache with 10,000 entry capacity
- Property-based testing with proptest (10 tests)
- Unit tests for core functionality (6 tests)
- Cross-compilation support for ARM64 routers
- Basic cache-control header support
- Transparent proxy on port 3128
- Single-threaded tokio runtime for low memory usage

### Known Issues
- Memory limits not enforced (can OOM on large responses)
- Uses blocking I/O in async context (println!)
- Missing proper error handling (unwrap calls)
- No request size limits
- No logging rotation
- No metrics endpoint

### Planned for 0.1.0
- [ ] Memory safety improvements (Phase 1)
- [ ] Request safety limits (Phase 2)
- [ ] Complete error handling (Phase 3)
- [ ] Async structured logging (Phase 4)
- [ ] Binary size < 500KB
- [ ] Memory usage < 10MB idle
- [ ] Zero panics guarantee

## [0.1.0-alpha.1] - TBD

### Added
- Pre-release for testing safety improvements
- Memory limit enforcement
- Basic error handling improvements

## Version History

### Roadmap
- `0.1.0-alpha.X` - Safety improvements testing
- `0.1.0-beta.X` - Performance optimizations
- `0.1.0-rc.X` - Release candidates
- `0.1.0` - First stable release
- `0.2.0` - HTTP/2 support
- `0.3.0` - Distributed caching
- `1.0.0` - Production ready with full Squid feature parity (subset)