# Changelog

All notable changes to LLM Archive V2 will be documented in this file.

## [2.0.0] - 2025-01-15

### Added
- Complete rewrite in Rust for 60-100x performance improvement
- Native parsers for ChatGPT, Claude, Gemini, XAI, and Zed formats
- Python bridge via PyO3 for gradual migration from V1 parsers
- SQLite FTS5 full-text search with sub-100ms response times
- Comprehensive metrics collection with Prometheus format
- Askama templates with keyboard navigation
- Dark mode support
- Import event logging for audit trail
- Raw JSON storage for future schema migrations
- Background database statistics updater
- Responsive web UI optimized for desktop

### Changed
- Reduced database schema from 27 tables to 4 essential tables
- Single binary deployment (~15MB) vs Python's 300MB+ memory usage
- All indexes created upfront (V1 was missing critical indexes)
- Batch processing imports 1000 conversations at a time
- Removed unnecessary abstractions and over-engineering

### Performance Improvements
- Cold start: < 50ms (was 3-5 seconds)
- Search: < 100ms (was broken in V1)
- Memory usage: < 20MB (was 300MB+)
- Import speed: ~10k messages/sec (was ~100/sec)

### Technical Details
- Built with Axum web framework
- SQLite with WAL mode and optimized settings
- Streaming imports with rollback capability
- Metrics exposed at /metrics endpoint
- Health check at /health
- Static file serving with compression

## [1.0.0] - Previous Version

The original Python implementation with Django, which suffered from:
- 27 over-normalized database tables
- Missing critical indexes
- 600+ line class files with abstract factories
- 10+ second page loads
- Broken search functionality
- No metrics or observability