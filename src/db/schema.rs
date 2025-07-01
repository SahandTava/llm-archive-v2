/// Database schema - simplified from V1's 27 tables to just what we need
pub const CREATE_TABLES: &str = r#"
-- Providers table
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

-- Conversations table with all relevant metadata
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL,
    external_id TEXT,
    title TEXT,
    model TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Store raw JSON for future migrations
    raw_json TEXT,
    
    -- Additional metadata fields
    system_prompt TEXT,
    temperature REAL,
    max_tokens INTEGER,
    user_id TEXT,
    
    -- Unique constraint to prevent duplicate imports
    UNIQUE(provider, external_id)
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    conversation_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    model TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Additional metadata
    tokens INTEGER,
    finish_reason TEXT,
    tool_calls TEXT, -- JSON
    attachments TEXT, -- JSON
    
    FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- Import event log for audit trail (as suggested in review)
CREATE TABLE IF NOT EXISTS import_events (
    id INTEGER PRIMARY KEY,
    event_type TEXT NOT NULL,
    provider TEXT NOT NULL,
    file_path TEXT,
    status TEXT NOT NULL,
    stats TEXT, -- JSON with counts
    error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert default providers
INSERT OR IGNORE INTO providers (name) VALUES 
    ('chatgpt'), ('claude'), ('gemini'), ('xai'), ('zed');
"#;

/// FTS5 virtual table for blazing fast search
pub const CREATE_FTS: &str = r#"
-- Drop if exists to allow schema updates
DROP TABLE IF EXISTS messages_fts;

-- Create FTS5 table for full-text search
CREATE VIRTUAL TABLE messages_fts USING fts5(
    content,
    conversation_id UNINDEXED,
    
    -- Store additional searchable fields
    role UNINDEXED,
    
    -- Use Porter tokenizer for better stemming
    tokenize = 'porter'
);

-- Populate FTS from existing messages
INSERT OR IGNORE INTO messages_fts (rowid, content, conversation_id, role)
SELECT id, content, conversation_id, role FROM messages;

-- Create triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
BEGIN
    INSERT INTO messages_fts (rowid, content, conversation_id, role)
    VALUES (new.id, new.content, new.conversation_id, new.role);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages
BEGIN
    UPDATE messages_fts 
    SET content = new.content, role = new.role
    WHERE rowid = new.id;
END;
"#;

/// Essential indexes for performance
pub const CREATE_INDEXES: &str = r#"
-- Conversation indexes
CREATE INDEX IF NOT EXISTS idx_conversations_created_at 
ON conversations(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_provider 
ON conversations(provider);

CREATE INDEX IF NOT EXISTS idx_conversations_model 
ON conversations(model);

CREATE INDEX IF NOT EXISTS idx_conversations_user_id 
ON conversations(user_id);

-- Message indexes
CREATE INDEX IF NOT EXISTS idx_messages_conversation_id 
ON messages(conversation_id);

CREATE INDEX IF NOT EXISTS idx_messages_created_at 
ON messages(created_at);

CREATE INDEX IF NOT EXISTS idx_messages_role 
ON messages(role);

-- Import event indexes
CREATE INDEX IF NOT EXISTS idx_import_events_created_at 
ON import_events(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_import_events_provider 
ON import_events(provider);
"#;