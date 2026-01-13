use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

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

pub struct SemanticChunker {
    max_chunk_size: usize,
    min_chunk_size: usize,
    overlap: usize,
}

impl SemanticChunker {
    pub fn new(max_chunk_size: usize, min_chunk_size: usize, overlap: usize) -> Self {
        Self {
            max_chunk_size,
            min_chunk_size,
            overlap,
        }
    }

    pub fn chunk(&self, _code: &str, _language: Option<&str>) -> Result<Vec<Chunk>> {
        error!("SemanticChunker::chunk not implemented yet");
        anyhow::bail!("SemanticChunker::chunk not implemented yet");
    }
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self::new(512, 128, 50)
    }
}
