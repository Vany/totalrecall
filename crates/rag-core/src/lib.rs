use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: MemoryMetadata,
    pub scope: MemoryScope,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

impl Memory {
    pub fn new(content: String, scope: MemoryScope, metadata: MemoryMetadata) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            embedding: Vec::new(),
            metadata,
            scope,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub tags: Vec<String>,
    pub source_file: Option<PathBuf>,
    pub language: Option<String>,
    pub chunk_index: Option<usize>,
    pub parent_id: Option<String>,
    pub ast_node_type: Option<String>,
    pub importance_score: f32,
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for MemoryMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            source_file: None,
            language: None,
            chunk_index: None,
            parent_id: None,
            ast_node_type: None,
            importance_score: 1.0,
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryScope {
    Session,
    Project { path: PathBuf },
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub content: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub ast_context: Option<AstContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstContext {
    pub node_type: String,
    pub parent_types: Vec<String>,
    pub depth: usize,
    pub is_declaration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub memory: Memory,
    pub score: f32,
    pub rank: usize,
}
