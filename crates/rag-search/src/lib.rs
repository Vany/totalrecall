use rag_core::{Memory, SearchResult};
use regex::Regex;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

pub struct BM25SearchEngine {
    k1: f32,
    b: f32,
    avg_doc_length: f32,
    doc_count: usize,
    doc_lengths: HashMap<String, usize>,
    term_doc_freq: HashMap<String, usize>,
    stop_words: Vec<String>,
}

impl BM25SearchEngine {
    pub fn new() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            avg_doc_length: 0.0,
            doc_count: 0,
            doc_lengths: HashMap::new(),
            term_doc_freq: HashMap::new(),
            stop_words: Self::default_stop_words(),
        }
    }

    fn default_stop_words() -> Vec<String> {
        vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "is", "was", "are", "were", "be", "been", "being", "have", "has", "had", "do", "does",
            "did", "will", "would", "could", "should", "may", "might", "can", "this", "that",
            "these", "those",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        let re = Regex::new(r"[^\w\s]").unwrap();
        let cleaned = re.replace_all(text, " ");

        cleaned
            .unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 1 && !self.stop_words.contains(w))
            .collect()
    }

    pub fn index_memory(&mut self, memory: &Memory) {
        let tokens = self.tokenize(&memory.content);
        let doc_len = tokens.len();

        self.doc_lengths.insert(memory.id.clone(), doc_len);
        self.doc_count += 1;

        let mut unique_terms = std::collections::HashSet::new();
        for token in &tokens {
            unique_terms.insert(token.clone());
        }

        for term in unique_terms {
            *self.term_doc_freq.entry(term).or_insert(0) += 1;
        }

        let total_length: usize = self.doc_lengths.values().sum();
        self.avg_doc_length = total_length as f32 / self.doc_count as f32;
    }

    pub fn search(&self, query: &str, memories: &[Memory], k: usize) -> Vec<SearchResult> {
        let query_tokens = self.tokenize(query);
        let mut scores: Vec<(usize, f32)> = Vec::new();

        for (idx, memory) in memories.iter().enumerate() {
            let score = self.score_document(memory, &query_tokens);
            if score > 0.0 {
                scores.push((idx, score));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        scores
            .into_iter()
            .take(k)
            .enumerate()
            .map(|(rank, (idx, score))| SearchResult {
                memory: memories[idx].clone(),
                score,
                rank,
            })
            .collect()
    }

    fn score_document(&self, memory: &Memory, query_tokens: &[String]) -> f32 {
        let doc_tokens = self.tokenize(&memory.content);
        let doc_len = self
            .doc_lengths
            .get(&memory.id)
            .copied()
            .unwrap_or(doc_tokens.len());

        let mut term_freq: HashMap<String, usize> = HashMap::new();
        for token in &doc_tokens {
            *term_freq.entry(token.clone()).or_insert(0) += 1;
        }

        let mut score = 0.0;

        for query_term in query_tokens {
            let tf = *term_freq.get(query_term).unwrap_or(&0) as f32;

            if tf == 0.0 {
                continue;
            }

            let df = *self.term_doc_freq.get(query_term).unwrap_or(&0) as f32;
            let idf = ((self.doc_count as f32 - df + 0.5) / (df + 0.5) + 1.0).ln();

            let norm = 1.0 - self.b + self.b * (doc_len as f32 / self.avg_doc_length.max(1.0));
            let tf_norm = (tf * (self.k1 + 1.0)) / (tf + self.k1 * norm);

            score += idf * tf_norm;
        }

        score
    }

    pub fn remove_memory(&mut self, memory_id: &str) {
        if self.doc_lengths.remove(memory_id).is_some() {
            self.doc_count = self.doc_count.saturating_sub(1);

            if self.doc_count > 0 {
                let total_length: usize = self.doc_lengths.values().sum();
                self.avg_doc_length = total_length as f32 / self.doc_count as f32;
            } else {
                self.avg_doc_length = 0.0;
            }
        }
    }

    pub fn reindex_all(&mut self, memories: &[Memory]) {
        self.doc_lengths.clear();
        self.term_doc_freq.clear();
        self.doc_count = 0;
        self.avg_doc_length = 0.0;

        for memory in memories {
            self.index_memory(memory);
        }
    }
}

impl Default for BM25SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
