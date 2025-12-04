use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub response_body: Vec<u8>,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub created_at: u64,
    pub ttl_seconds: u64,
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.created_at + self.ttl_seconds
    }
}

pub struct CacheManager {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_entries: usize,
    default_ttl: u64,
}

impl CacheManager {
    pub fn new(max_entries: usize, default_ttl: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            default_ttl,
        }
    }
    
    /// 生成缓存 Key (基于路径和请求体的 SHA256)
    pub fn generate_key(path: &str, body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        hasher.update(body);
        format!("{:x}", hasher.finalize())
    }
    
    /// 获取缓存
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        let cache = self.cache.read().ok()?;
        let entry = cache.get(key)?;
        
        if entry.is_expired() {
            // 过期了，返回 None（下次写入时会覆盖）
            None
        } else {
            Some(entry.clone())
        }
    }
    
    /// 设置缓存
    pub fn set(&self, key: String, response_body: Vec<u8>, status: u16, headers: Vec<(String, String)>) {
        let mut cache = match self.cache.write() {
            Ok(c) => c,
            Err(_) => return,
        };
        
        // 如果超过最大条目数，清理过期条目
        if cache.len() >= self.max_entries {
            self.evict_expired_internal(&mut cache);
            
            // 如果还是满了，删除最旧的
            if cache.len() >= self.max_entries {
                // 简单策略：删除第一个找到的
                if let Some(k) = cache.keys().next().cloned() {
                    cache.remove(&k);
                }
            }
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        cache.insert(key, CacheEntry {
            response_body,
            status,
            headers,
            created_at: now,
            ttl_seconds: self.default_ttl,
        });
    }
    
    /// 清理过期条目
    pub fn evict_expired(&self) {
        if let Ok(mut cache) = self.cache.write() {
            self.evict_expired_internal(&mut cache);
        }
    }
    
    fn evict_expired_internal(&self, cache: &mut HashMap<String, CacheEntry>) {
        cache.retain(|_, entry| !entry.is_expired());
    }
    
    /// 清空所有缓存
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
    
    /// 获取缓存统计
    pub fn stats(&self) -> (usize, usize) {
        let cache = match self.cache.read() {
            Ok(c) => c,
            Err(_) => return (0, self.max_entries),
        };
        (cache.len(), self.max_entries)
    }
}

impl Clone for CacheManager {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            max_entries: self.max_entries,
            default_ttl: self.default_ttl,
        }
    }
}
