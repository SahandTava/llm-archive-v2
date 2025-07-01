use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,
    
    #[serde(default)]
    pub search: SearchConfig,
    
    #[serde(default)]
    pub import: ImportConfig,
    
    #[serde(default)]
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
    
    #[serde(default = "default_true")]
    pub wal_mode: bool,
    
    #[serde(default = "default_mmap_size")]
    pub mmap_size: u64,
    
    #[serde(default = "default_cache_size")]
    pub cache_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    
    #[serde(default = "default_snippet_length")]
    pub snippet_length: usize,
    
    #[serde(default = "default_true")]
    pub highlight_matches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    
    #[serde(default = "default_true")]
    pub python_bridge: bool,
    
    #[serde(default = "default_false")]
    pub skip_duplicates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    
    #[serde(default = "default_host")]
    pub host: String,
    
    #[serde(default = "default_static_dir")]
    pub static_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            search: SearchConfig::default(),
            import: ImportConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
            wal_mode: true,
            mmap_size: default_mmap_size(),
            cache_size: default_cache_size(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: default_max_results(),
            snippet_length: default_snippet_length(),
            highlight_matches: true,
        }
    }
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            batch_size: default_batch_size(),
            python_bridge: true,
            skip_duplicates: false,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            static_dir: default_static_dir(),
        }
    }
}

impl Config {
    /// Load configuration from file or use defaults
    pub fn load() -> Result<Self> {
        // Check for config file in standard locations
        let config_paths = [
            "./config.toml",
            "./llm-archive.toml",
            "~/.config/llm-archive/config.toml",
        ];
        
        for path in &config_paths {
            let expanded = shellexpand::tilde(path);
            let path = Path::new(expanded.as_ref());
            
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        
        // No config file found, use defaults
        Ok(Config::default())
    }
    
    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}

// Default value functions
fn default_db_path() -> String {
    "./llm_archive.db".to_string()
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_mmap_size() -> u64 {
    1_073_741_824 // 1GB
}

fn default_cache_size() -> i32 {
    -64000 // 64MB in pages
}

fn default_max_results() -> usize {
    100
}

fn default_snippet_length() -> usize {
    200
}

fn default_batch_size() -> usize {
    1000
}

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_static_dir() -> String {
    "./static".to_string()
}