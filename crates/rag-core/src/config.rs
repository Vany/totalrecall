use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub search: SearchConfig,
    pub chunking: ChunkingConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_k")]
    pub default_k: usize,
    #[serde(default = "default_min_score")]
    pub min_score: f32,
    #[serde(default = "default_bm25_k1")]
    pub bm25_k1: f32,
    #[serde(default = "default_bm25_b")]
    pub bm25_b: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    #[serde(default = "default_max_chunk_size")]
    pub max_chunk_size: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_global_db_path")]
    pub global_db_path: PathBuf,
    #[serde(default = "default_project_db_name")]
    pub project_db_name: String,
    #[serde(default = "default_max_session_memories")]
    pub max_session_memories: usize,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_k() -> usize {
    5
}

fn default_min_score() -> f32 {
    0.0
}

fn default_bm25_k1() -> f32 {
    1.2
}

fn default_bm25_b() -> f32 {
    0.75
}

fn default_max_chunk_size() -> usize {
    512
}

fn default_chunk_overlap() -> usize {
    50
}

fn default_global_db_path() -> PathBuf {
    // Allow override via environment variable (for testing)
    if let Ok(path) = std::env::var("RAG_MCP_DB_PATH") {
        return PathBuf::from(path).join("global.db");
    }

    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rag-mcp")
        .join("global.db")
}

fn default_project_db_name() -> String {
    ".rag-mcp/data.db".to_string()
}

fn default_max_session_memories() -> usize {
    1000
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                log_level: default_log_level(),
            },
            search: SearchConfig {
                default_k: default_k(),
                min_score: default_min_score(),
                bm25_k1: default_bm25_k1(),
                bm25_b: default_bm25_b(),
            },
            chunking: ChunkingConfig {
                max_chunk_size: default_max_chunk_size(),
                chunk_overlap: default_chunk_overlap(),
            },
            storage: StorageConfig {
                global_db_path: default_global_db_path(),
                project_db_name: default_project_db_name(),
                max_session_memories: default_max_session_memories(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let contents =
                std::fs::read_to_string(&config_path).context("Failed to read config file")?;
            let config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;

        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rag-mcp")
            .join("config.toml")
    }
}
