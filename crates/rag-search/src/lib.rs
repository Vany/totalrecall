use anyhow::Result;
use rag_core::{Memory, SearchResult};
use tracing::error;

pub struct HybridSearchEngine {
    _dimension: usize,
}

impl HybridSearchEngine {
    pub fn new(dimension: usize) -> Result<Self> {
        error!("HybridSearchEngine not implemented yet");
        Ok(Self { _dimension: dimension })
    }

    pub async fn search(&self, _query_embedding: &[f32], _k: usize) -> Result<Vec<SearchResult>> {
        error!("HybridSearchEngine::search not implemented yet");
        anyhow::bail!("HybridSearchEngine::search not implemented yet");
    }

    pub async fn index(&self, _memory: &Memory) -> Result<()> {
        error!("HybridSearchEngine::index not implemented yet");
        anyhow::bail!("HybridSearchEngine::index not implemented yet");
    }
}
