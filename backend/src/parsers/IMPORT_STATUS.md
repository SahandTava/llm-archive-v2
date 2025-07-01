# Import Status Report [TAG: IMP]

## V1 Parser Analysis

### Key Issues Identified

#### 1. Timestamp Handling
- **Zed Provider**: Uses file modification time as fallback - no actual timestamps preserved from conversations
  - Attempts to interpolate timestamps across 1-hour duration (line 126-135 in zed.py)
  - This is a fundamental limitation of Zed's export format
- **Gemini Provider**: Complex timestamp parsing with multiple fallback strategies
  - Handles millisecond timestamps, ISO formats, and various string formats
  - Falls back to current time if parsing fails
- **ChatGPT Provider**: Good timestamp handling but could be more robust
- **Claude Provider**: Decent timestamp parsing but lacks microsecond precision

#### 2. System Prompt Handling
- **Zed Provider**: No system prompt extraction at all
- **Gemini Provider**: No system prompt extraction - messages start with user/assistant only
- **ChatGPT Provider**: Properly extracts system prompts (lines 220-224, 424-428)
- **Claude Provider**: System prompt extraction implemented (lines 350-355)

#### 3. Role Mapping
- **Zed Provider**: Simple role mapping from metadata, defaults to "unknown"
- **Gemini Provider**: Complex role mapping with multiple fallbacks
  - Maps "human" → "user", "model" → "assistant"
  - Handles various format-specific role keys
- **ChatGPT Provider**: Good role mapping with support for system/tool roles
- **Claude Provider**: Maps "human" → "user", handles assistant/model roles

#### 4. Error Handling & Logging
- Inconsistent error handling across providers
- Some providers silently skip errors, others log warnings
- No structured error collection for reporting back to users

#### 5. Media/Attachment Handling
- **Zed Provider**: Basic media reference detection via regex
- **Gemini Provider**: No media handling implemented
- **ChatGPT Provider**: No media handling implemented
- **Claude Provider**: Comprehensive attachment handling with extracted content

#### 6. Performance Concerns
- Large file parsing happens in-memory
- No streaming support for large exports
- Duplicate message detection happens post-import

## V2 Improvements Plan

### 1. Unified Timestamp Handling
- Create a robust timestamp parser that handles all known formats
- Store original timestamp strings for debugging
- Use high-precision timestamps (nanoseconds) where available
- Implement proper timezone handling

### 2. Comprehensive System Prompt Support
- Extract system prompts from all providers where available
- Store as separate field in conversation metadata
- Allow reconstruction of full conversation context

### 3. Standardized Role Mapping
- Define canonical roles: user, assistant, system, tool
- Create provider-specific mapping tables
- Log unmapped roles for analysis

### 4. Structured Error Collection
- Collect all parsing errors/warnings
- Return structured error report with import results
- Allow users to see what data couldn't be imported

### 5. Streaming Parser Architecture
- Implement streaming JSON parsing for large files
- Process conversations as they're parsed
- Reduce memory footprint for large imports

### 6. Enhanced Media Handling
- Extract and store media metadata
- Support for embedded media (base64)
- Track media references for later download

### 7. Import Validation
- Validate required fields before import
- Check for data integrity issues
- Provide detailed import statistics

## Provider-Specific Notes

### ChatGPT
- Best timestamp support among V1 parsers
- Handles multiple export formats (mapping, message array)
- System prompt extraction works well

### Claude
- Good attachment handling with extracted content
- Proper system prompt support
- UUID tracking for deduplication

### Gemini
- Most complex parser due to multiple format variants
- Needs better structure for maintainability
- HTML parsing adds complexity

### Zed
- Most problematic due to lack of timestamps
- Would benefit from Zed API integration for proper data
- Current interpolation is a hack at best

## Implementation Priority

1. **Common utilities** - Shared timestamp parsing, role mapping ✅ COMPLETED
2. **ChatGPT parser** - Most straightforward to implement ✅ COMPLETED
3. **Claude parser** - Good reference implementation ✅ COMPLETED
4. **Gemini parser** - Needs significant refactoring ⚠️ STUB CREATED
5. **Zed parser** - Consider if worth supporting without proper timestamps ⚠️ STUB CREATED

## V2 Implementation Status

### Completed Components

1. **Parser Framework** (`parsers/mod.rs`)
   - Unified trait system for all providers
   - Structured error handling with context
   - Import statistics collection
   - Provider registry for automatic detection

2. **Common Utilities** (`parsers/common.rs`)
   - Robust timestamp parsing (ISO 8601, numeric, various formats)
   - Text content extraction from nested JSON structures
   - Media reference detection via regex
   - Metadata sanitization and validation
   - MIME type detection

3. **ChatGPT Parser** (`parsers/chatgpt.rs`)
   - Handles both mapping and message array formats
   - Proper system prompt extraction
   - Recursive tree traversal for mapping format
   - Content type handling (text, code, browsing)
   - Error context preservation

4. **Claude Parser** (`parsers/claude.rs`)
   - Bulk export (conversations.json) support
   - Single conversation file support
   - Attachment processing with extracted content
   - System prompt extraction
   - UUID-based conversation IDs

### Key Improvements Over V1

1. **Type Safety**: Rust's type system prevents many runtime errors
2. **Async Support**: Non-blocking I/O for better performance
3. **Structured Errors**: Detailed error reporting with context
4. **Memory Efficiency**: Streaming-ready architecture
5. **Standardized Data**: Consistent conversation/message format
6. **Comprehensive Testing**: Unit tests for core functionality

### Outstanding Work

1. **Gemini Parser**: Complex due to multiple format variants
   - MyActivity.json with HTML parsing
   - Multiple timestamp formats and fallbacks
   - Audio/image handling in chunkedPrompt format
   - Different role mappings per format

2. **Zed Parser**: Fundamental timestamp limitations
   - Only file modification time available
   - Position-based message extraction from full text
   - Consider dropping support or requiring API access

3. **Integration**: Connect to V2 backend
   - Database schema mapping
   - API endpoint integration
   - Batch import workflow

4. **Performance**: Large file handling
   - Streaming JSON parsing
   - Memory usage optimization
   - Progress reporting

### Files Created

- `/backend/src/parsers/mod.rs` - Main module with traits and types
- `/backend/src/parsers/common.rs` - Shared utility functions
- `/backend/src/parsers/chatgpt.rs` - Complete ChatGPT implementation
- `/backend/src/parsers/claude.rs` - Complete Claude implementation  
- `/backend/src/parsers/gemini.rs` - Stub for future implementation
- `/backend/src/parsers/zed.rs` - Stub for future implementation

### Next Steps

1. Implement Gemini parser based on V1 analysis
2. Evaluate Zed parser necessity
3. Add integration tests with real export files
4. Connect to V2 backend database
5. Add progress reporting and cancellation support