use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// High-performance in-memory cache for frequently accessed data
/// Uses LRU eviction with time-based expiration
pub struct SmartCache<K, V> {
    cache: Arc<RwLock<LruCache<K, CachedItem<V>>>>,
    ttl: Duration,
}

struct CachedItem<V> {
    value: V,
    expires_at: Instant,
}

impl<K: std::hash::Hash + Eq, V: Clone> SmartCache<K, V> {
    pub fn new(capacity: usize, ttl_seconds: u64) -> Self {
        let cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
        Self {
            cache: Arc::new(RwLock::new(cache)),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get item from cache if not expired
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;
        
        if let Some(item) = cache.get_mut(key) {
            if item.expires_at > Instant::now() {
                return Some(item.value.clone());
            } else {
                // Remove expired item
                cache.pop(key);
            }
        }
        None
    }

    /// Insert item with TTL
    pub async fn insert(&self, key: K, value: V) {
        let expires_at = Instant::now() + self.ttl;
        let item = CachedItem { value, expires_at };
        
        let mut cache = self.cache.write().await;
        cache.put(key, item);
    }

    /// Clear all expired entries
    pub async fn evict_expired(&self) {
        let now = Instant::now();
        let mut cache = self.cache.write().await;
        
        // Collect expired keys
        let expired: Vec<K> = cache
            .iter()
            .filter(|(_, item)| item.expires_at <= now)
            .map(|(k, _)| k.clone())
            .collect();
        
        // Remove expired entries
        for key in expired {
            cache.pop(&key);
        }
    }
}

/// Specialized cache for search results
pub struct SearchCache {
    cache: SmartCache<String, Vec<SearchResult>>,
}

#[derive(Clone)]
pub struct SearchResult {
    pub conversation_id: i64,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

impl SearchCache {
    pub fn new() -> Self {
        // Cache up to 1000 search results for 5 minutes
        Self {
            cache: SmartCache::new(1000, 300),
        }
    }

    pub async fn get_results(&self, query: &str) -> Option<Vec<SearchResult>> {
        self.cache.get(&query.to_lowercase()).await
    }

    pub async fn cache_results(&self, query: &str, results: Vec<SearchResult>) {
        self.cache.insert(query.to_lowercase(), results).await;
    }
}

/// Specialized cache for conversation data
pub struct ConversationCache {
    cache: SmartCache<i64, CachedConversation>,
}

#[derive(Clone)]
pub struct CachedConversation {
    pub id: i64,
    pub title: String,
    pub message_count: usize,
    pub provider: String,
    pub messages_preview: Vec<MessagePreview>,
}

#[derive(Clone)]
pub struct MessagePreview {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

impl ConversationCache {
    pub fn new() -> Self {
        // Cache up to 500 conversations for 10 minutes
        Self {
            cache: SmartCache::new(500, 600),
        }
    }

    pub async fn get(&self, id: i64) -> Option<CachedConversation> {
        self.cache.get(&id).await
    }

    pub async fn insert(&self, conversation: CachedConversation) {
        self.cache.insert(conversation.id, conversation).await;
    }
}

/// Background task to periodically evict expired entries
pub async fn cache_maintenance_task(
    search_cache: Arc<SearchCache>,
    conv_cache: Arc<ConversationCache>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        // Evict expired entries
        search_cache.cache.evict_expired().await;
        conv_cache.cache.evict_expired().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let cache: SmartCache<String, String> = SmartCache::new(10, 1);
        
        cache.insert("key".to_string(), "value".to_string()).await;
        
        // Should be present immediately
        assert_eq!(cache.get(&"key".to_string()).await, Some("value".to_string()));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Should be expired
        assert_eq!(cache.get(&"key".to_string()).await, None);
    }
    
    #[tokio::test]
    async fn test_lru_eviction() {
        let cache: SmartCache<i32, i32> = SmartCache::new(2, 60);
        
        cache.insert(1, 1).await;
        cache.insert(2, 2).await;
        cache.insert(3, 3).await; // Should evict 1
        
        assert_eq!(cache.get(&1).await, None);
        assert_eq!(cache.get(&2).await, Some(2));
        assert_eq!(cache.get(&3).await, Some(3));
    }
}