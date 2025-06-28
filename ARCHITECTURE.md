# LLM Archive V2 Architecture

## Overview

LLM Archive V2 is a complete rewrite focused on performance, simplicity, and reliability. Built in Rust, it achieves sub-second search across millions of messages while using minimal resources.

## Key Design Decisions

### 1. Technology Stack

- **Rust + Axum**: Native performance, memory safety, single binary deployment
- **SQLite + FTS5**: Embedded database with full-text search, no external dependencies
- **PyO3 Bridge**: Temporary Python interop to reuse V1 parsers during migration
- **Askama Templates**: Type-safe, compiled templates for fast rendering

### 2. Database Schema (Simplified from 27 to 4 tables)

```sql
conversations
├── id (primary key)
├── provider (chatgpt, claude, etc.)
├── external_id (unique with provider)
├── title
├── model
├── created_at, updated_at
├── raw_json (for future migrations)
└── metadata (system_prompt, temperature, max_tokens, user_id)

messages
├── id (primary key)  
├── conversation_id (foreign key)
├── role (user, assistant, system, tool)
├── content
├── model (can vary per message)
├── created_at
└── metadata (tokens, finish_reason, tool_calls, attachments)

messages_fts (virtual FTS5 table)
├── content (searchable)
├── conversation_id (unindexed)
└── role (unindexed)

import_events (audit trail)
├── id (primary key)
├── event_type, provider, file_path
├── status (in_progress, completed, failed)
├── stats (JSON with counts)
└── error (if failed)
```

### 3. Performance Optimizations

- **WAL Mode**: Better concurrency for reads during imports
- **Memory-mapped I/O**: Fast file access for large imports
- **Batch Processing**: Import in chunks of 1000 conversations
- **Prepared Statements**: Reuse SQL compilation
- **Connection Pooling**: Efficient database connections
- **All Indexes from Day 1**: No missing indexes unlike V1

### 4. Import Pipeline

```
Export File → Parser → Domain Models → Batch Insert → FTS Index
     ↓                      ↓                ↓
  Python Bridge      Store raw_json    Event logging
  (temporary)        (migrations)      (audit trail)
```

### 5. Search Strategy

1. **Primary**: FTS5 with Porter stemming for natural language
2. **Filters**: Provider, model, date range, user
3. **Ranking**: SQLite's built-in BM25 algorithm
4. **Snippets**: Context around matches with highlighting

### 6. Module Structure

```
src/
├── main.rs              # CLI entry point
├── config.rs            # Configuration management
├── errors.rs            # Error types and handling
├── models.rs            # Domain models
├── db/
│   ├── mod.rs          # Database connection
│   └── schema.rs       # SQL schema definitions
├── import/
│   ├── mod.rs          # Import orchestration
│   ├── python_bridge.rs # PyO3 integration
│   └── parsers/        # Native Rust parsers
│       ├── chatgpt.rs
│       └── claude.rs
├── search.rs           # Search implementation
└── server/
    ├── mod.rs          # Web server setup
    └── templates.rs    # HTML rendering
```

## Migration Strategy

### Phase 1: Python Bridge (Current)
- Use PyO3 to call existing Python parsers
- Validate data integrity
- Store raw JSON for future migrations

### Phase 2: Native Parsers
- Implement Rust parsers for each provider
- Profile and optimize hot paths
- Remove Python dependency

### Phase 3: Advanced Features
- Streaming imports for huge files
- Incremental indexing
- Search query optimization

## Performance Targets vs V1

| Metric | V2 Target | V1 Actual | Improvement |
|--------|-----------|-----------|-------------|
| Cold Start | < 50ms | 3-5s | 60-100x |
| Search | < 100ms | Broken | N/A |
| Memory | < 20MB | 300MB+ | 15x |
| Import | 10k/sec | 100/sec | 100x |
| Binary Size | < 15MB | N/A | N/A |

## Security Considerations

1. **SQL Injection**: All queries use parameterized statements
2. **Path Traversal**: Validate all file paths
3. **Memory Safety**: Rust prevents buffer overflows
4. **DoS Protection**: Rate limiting on search endpoint
5. **Data Privacy**: No telemetry, local-only by default

## Future Enhancements

1. **WebAssembly Build**: Run in browser for demos
2. **gRPC API**: For programmatic access
3. **Plugins**: Custom parsers for new formats
4. **Federation**: Search across multiple instances
5. **ML Features**: Semantic search, clustering

## Lessons from V1

### What V1 Did Wrong
- 27 tables for simple data (over-normalized)
- Abstract factories and 600-line classes
- Missing database indexes
- No performance profiling
- Feature creep (LDAP auth, etc.)

### What V2 Does Right
- 4 tables, flat structure
- Direct, simple code paths
- Indexes created upfront
- Performance benchmarks
- Core features only

## Development Guidelines

1. **Keep It Simple**: No abstractions until proven necessary
2. **Profile First**: Measure before optimizing
3. **Test Real Data**: Use actual export files
4. **Document Decisions**: Update this file
5. **User First**: Fast search is the #1 priority