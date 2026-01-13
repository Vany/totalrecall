use anyhow::Result;
use tracing::error;

pub struct BertEmbedder {
    dimension: usize,
}

impl BertEmbedder {
    pub fn new() -> Result<Self> {
        error!("BertEmbedder not implemented yet");
        Ok(Self { dimension: 768 })
    }

    pub fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        error!("BertEmbedder::embed not implemented yet");
        anyhow::bail!("BertEmbedder::embed not implemented yet");
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }
}
