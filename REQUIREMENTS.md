# LLM Archive V2 Requirements
[TAG: REQ]

## Executive Summary

LLM Archive V2 is a focused tool for searching and viewing LLM conversation exports. Based on V1 failures and user feedback, V2 will prioritize speed, reliability, and desktop usability over complex features that users don't need.

**Core Mission**: Enable users to quickly find and read their AI conversations without lag or complexity.

---

## 1. Core Functional Requirements

### 1.1 Import (MUST HAVE)
- **Supported Formats**:
  - ChatGPT JSON exports (90% of usage)
  - Claude JSON exports
  - Gemini exports
  - XAI exports (if simple)
  - Zed exports (if simple)
  
- **Import Performance**:
  - Process 10,000 messages in <10 seconds
  - Stream parsing for files >100MB
  - Show real-time progress indicator
  - No blocking of web interface during import

- **Import Accuracy**:
  - Preserve exact timestamps
  - Maintain message ordering
  - Handle code blocks correctly
  - Preserve system prompts
  - Extract model information dynamically

### 1.2 Search (MUST HAVE)
- **Search Performance**:
  - First results appear in <100ms
  - Full search results in <1 second for 100k+ messages
  - No timeouts or freezing
  
- **Search Features**:
  - Full-text search using FTS5 only
  - Search in message content
  - Case-insensitive by default
  - NO semantic/ML search
  - NO complex search strategies

- **Search UI**:
  - Single search box
  - Results appear immediately
  - Show context around matches
  - Highlight search terms

### 1.3 View Conversations (MUST HAVE)
- **List View**:
  - Show conversations sorted by date (newest first)
  - Display title, date, message count
  - Pagination (50-100 per page)
  - Load in <500ms
  
- **Conversation View**:
  - Display all messages in order
  - Show role (user/assistant/system)
  - Preserve formatting (code, lists, etc.)
  - Collapse/expand long messages
  - Load in <500ms even for 1000+ messages

### 1.4 Basic Filtering (SHOULD HAVE)
- Filter by date range
- Filter by provider (ChatGPT, Claude, etc.)
- Filter by model (if detected)
- Filters apply instantly (<100ms)

### 1.5 Export (SHOULD HAVE)
- Export single conversation as:
  - Markdown
  - JSON (original format)
  - Plain text
- Export search results
- Complete in <5 seconds for typical conversation

---

## 2. Performance Requirements

### 2.1 Startup Performance
- **Cold Start**: <1 second to serving requests
- **Warm Start**: <200ms
- **Binary Size**: <20MB (if compiled)
- **Memory Usage**: <100MB baseline, <200MB under load

### 2.2 Runtime Performance
- **Homepage Load**: <300ms
- **Search Response**: <100ms for first results, <1s for complete
- **Conversation List**: <500ms for 100 items with pagination
- **Conversation View**: <500ms for any size conversation
- **Database Queries**: All queries <100ms

### 2.3 Scalability Limits
- Support up to 1M messages
- Handle conversations with 10k+ messages
- Search across entire corpus without degradation
- Single-user desktop application (no multi-user requirements)

---

## 3. UI/UX Requirements

### 3.1 Desktop-First Design
- **Fixed Width**: 1200px optimized for desktop screens
- **No Mobile Support**: Explicitly desktop-only
- **Dense Information Display**: Show more data, less whitespace
- **Keyboard Navigation**: j/k for up/down, / for search

### 3.2 Visual Design
- **Simple and Fast**: Minimal CSS, no animations
- **High Contrast**: Easy to read for long sessions
- **Fixed Layout**: No responsive breakpoints
- **Native Feel**: Like a desktop application, not a web app

### 3.3 Core UI Elements
- **Search Bar**: Always visible at top
- **Navigation**: Simple sidebar or top nav
- **Content Area**: Maximum space for conversations
- **No Modals**: Everything inline
- **No Loading Spinners**: Operations too fast to need them

---

## 4. Technical Requirements

### 4.1 Database
- **SQLite** with FTS5 enabled
- **Schema**: Maximum 5 tables:
  - providers (ChatGPT, Claude, etc.)
  - conversations (sessions)
  - messages (individual messages)
  - messages_fts (search index)
  - settings (optional)
- **Indexes**: On all foreign keys and commonly queried fields
- **No ORM**: Direct SQL for performance

### 4.2 Architecture
- **Single Process**: No background workers or queues
- **Synchronous**: No async complexity unless required
- **Direct Operations**: No repository/service layers
- **Minimal Dependencies**: Core language stdlib preferred

### 4.3 Code Constraints  
- **Total LOC**: <2,000 lines excluding tests
- **No Patterns**: No Repository, Strategy, or Factory patterns
- **No Abstractions**: Direct implementation only
- **Clear Code**: Optimize for readability over cleverness

---

## 5. Non-Functional Requirements

### 5.1 Reliability
- **No Data Loss**: All imports preserve original data
- **Crash Recovery**: Database transactions for safety
- **Error Messages**: Clear, actionable error messages
- **No Silent Failures**: All errors visible to user

### 5.2 Simplicity
- **No Configuration**: Works out of the box
- **No Setup Wizard**: Drop file, see results
- **No User Accounts**: Single-user desktop app
- **No Cloud Features**: Fully local operation

### 5.3 Compatibility
- **File Formats**: Handle format variations gracefully
- **Large Files**: Stream processing for >1GB exports
- **Special Characters**: Full Unicode support
- **Cross-Platform**: Windows, Mac, Linux

---

## 6. Explicit Non-Requirements

### 6.1 Features NOT to Build
- ❌ **Semantic/ML Search**: No embeddings, vectors, or AI
- ❌ **Deduplication**: No complex similarity detection
- ❌ **Real-time Sync**: No live updates from AI services
- ❌ **Collaboration**: Single-user only
- ❌ **Analytics**: No charts, graphs, or statistics
- ❌ **Themes**: No dark mode or customization
- ❌ **Plugins**: No extension system
- ❌ **API**: No REST endpoints needed
- ❌ **Background Processing**: Everything synchronous
- ❌ **Caching Layers**: Direct DB queries only

### 6.2 Patterns NOT to Use
- ❌ **Repository Pattern**: Direct DB access
- ❌ **Dependency Injection**: Direct imports
- ❌ **Event Systems**: Direct function calls
- ❌ **Middleware**: Minimal request processing
- ❌ **ORMs**: Raw SQL only
- ❌ **Async/Await**: Unless absolutely necessary

---

## 7. Success Criteria

### 7.1 Performance Metrics
- [ ] Cold start <1 second
- [ ] Search results appear <100ms
- [ ] 50k messages import <30 seconds
- [ ] Memory usage <100MB idle
- [ ] All pages load <500ms

### 7.2 Functionality Metrics
- [ ] Import ChatGPT exports without errors
- [ ] Import Claude exports without errors  
- [ ] Search returns relevant results
- [ ] Can view any conversation
- [ ] UI controls work as expected

### 7.3 Code Metrics
- [ ] <2,000 lines of application code
- [ ] <5 core dependencies
- [ ] <5 database tables
- [ ] 0 background processes
- [ ] 0 ML/AI libraries

---

## 8. User Validation Criteria

Based on V1 user complaints, V2 succeeds if:

1. **"No lag"** - Operations feel instant
2. **"Everything works"** - No broken features
3. **"Easy to use"** - No manual needed
4. **"Finds my conversations"** - Search actually works
5. **"Desktop focused"** - Optimized for desktop use

---

## 9. Testing Requirements

### 9.1 Functional Tests
- Import test files from each provider
- Search for known content
- Navigate all UI paths
- Verify data integrity

### 9.2 Performance Tests
- Measure all operation times
- Test with 100k+ messages
- Monitor memory usage
- Check query performance

### 9.3 User Acceptance Tests
- Import real export file
- Find specific conversation
- Read conversation fully
- Export conversation
- Complete workflow <2 minutes

---

## 10. Implementation Priorities

### Phase 1 (MVP - Week 1)
1. Database schema and setup
2. Import ChatGPT and Claude
3. Basic search with FTS5
4. List and view conversations
5. Desktop-optimized UI

### Phase 2 (Enhancement - Week 2)
1. Additional providers (Gemini, etc.)
2. Date/provider filtering  
3. Export functionality
4. Keyboard navigation
5. Performance optimization

### Out of Scope for V2
- Mobile responsive design
- Advanced analytics
- Semantic search
- Deduplication
- Multi-user support
- Cloud sync
- API development

---

## Summary

V2 succeeds by doing less, but doing it perfectly. Every feature must be fast, reliable, and desktop-optimized. No ML, no complexity, no patterns - just a fast, simple tool for searching conversations.

**Remember**: If the user can't see it or it doesn't make search faster, don't build it.

---

*Document created: June 28, 2025*
*Based on: V1 failure analysis, user feedback, performance data*
*Status: Ready for implementation*