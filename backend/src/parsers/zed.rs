// parsers/zed.rs - Zed AI export parser

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::{
    ChatProvider, Conversation, ImportStats, ParserResult,
};

/// Zed provider implementation
/// NOTE: Zed exports lack proper timestamps - uses file modification time as fallback
pub struct ZedProvider;

impl ZedProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ChatProvider for ZedProvider {
    fn name(&self) -> &'static str {
        "Zed"
    }

    async fn find_files(&self, _dir: &Path) -> ParserResult<Vec<PathBuf>> {
        // TODO: Implement file discovery
        // Should find individual JSON files with Zed structure:
        // - "zed": "context" marker
        // - "messages" array with start/end positions
        // - "text" field with full conversation text
        // - "id" and "version" fields
        Ok(Vec::new())
    }

    async fn can_handle(&self, _file: &Path) -> bool {
        // TODO: Implement format detection
        // Check for Zed-specific markers:
        // - Root object with "zed": "context"
        // - "messages" array with position-based structure
        // - "text" field containing full conversation
        false
    }

    async fn extract_conversations(
        &self,
        _file: &Path,
        _stats: &mut ImportStats,
    ) -> ParserResult<Vec<Conversation>> {
        // TODO: Implement extraction
        // Major challenges:
        // 1. No real timestamps - must use file mtime and interpolate
        // 2. Extract message content using start/end positions in full text
        // 3. Handle image_offsets for media references
        // 4. Role mapping from metadata
        // 5. Find media references in text using regex
        // 
        // Consider: Is this provider worth supporting given timestamp limitations?
        // Alternative: Suggest users export from Zed API if available
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zed_provider_basic() {
        let provider = ZedProvider::new();
        assert_eq!(provider.name(), "Zed");
    }
}