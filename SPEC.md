# RAG MCP Server for Zed/Claude Code - Technical Specification

here is also zed_plugins.md - zed documentation

## Project Overview

A production-grade Retrieval-Augmented Generation (RAG) system written in Rust, designed as an MCP (Model Context Protocol) server for Zed editor and Claude Code. This system provides semantic memory, code-aware chunking, and intelligent context retrieval for programming documentation and code understanding.

## Core Stack

### Dependencies
- **Storage**: `sahomedb` (v0.4.0) - Embedded vector database with HNSW indexing
- **Embeddings**: `candle-core`, `candle-nn`, `candle-transformers` - ML framework
- **BERT Model**: `all-mpnet-base-v2` (768 dimensions, better quality for code)
- **Tokenizer**: `tokenizers` - HuggingFace tokenizers
- **AST Parsing**: `tree-sitter` + language-specific grammars
- **MCP Protocol**: Custom JSON-RPC 2.0 over stdio
- **Serialization**: `serde`, `serde_json`, `bincode`
- **Config**: `toml`, `serde`
- **CLI**: `clap` v4
- **Logging**: `tracing`, `tracing-subscriber`
- **Async Runtime**: `tokio` v1.x

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Zed Editor                         │
│              (MCP Host Process)                     │
└──────────────────┬──────────────────────────────────┘
                   │ stdio (JSON-RPC 2.0)
                   │
┌──────────────────▼──────────────────────────────────┐
│              MCP Server Binary                       │
│  ┌────────────────────────────────────────────┐    │
│  │         MCP Protocol Handler               │    │
│  │  (JSON-RPC Request/Response Processing)    │    │
│  └────────────┬──────────────┬─────────────────┘    │
│               │              │                      │
│  ┌────────────▼─────┐  ┌────▼──────────────────┐   │
│  │  Tool Handlers   │  │  Resource Handlers    │   │
│  │  - store_memory  │  │  - memory://list      │   │
│  │  - search_memory │  │  - memory://stats     │   │
│  │  - update_memory │  │                       │   │
│  │  - delete_memory │  │                       │   │
│  │  - list_memories │  │                       │   │
│  │  - consolidate   │  │                       │   │
│  └────────────┬─────┘  └───────────────────────┘   │
│               │                                     │
│  ┌────────────▼─────────────────────────────────┐  │
│  │           RAG Engine Core                    │  │
│  │  ┌──────────────────────────────────────┐   │  │
│  │  │     Memory Manager                   │   │  │
│  │  │  - Session Memory (HashMap)          │   │  │
│  │  │  - Project Memory (Per-project DB)   │   │  │
│  │  │  - Global Memory (Shared DB)         │   │  │
│  │  └──────────────┬───────────────────────┘   │  │
│  │  ┌──────────────▼───────────────────────┐   │  │
│  │  │   Chunking & Embedding Pipeline      │   │  │
│  │  │  - AST-aware Semantic Chunker        │   │  │
│  │  │  - BERT Embedder (768-dim)           │   │  │
│  │  │  - Metadata Extractor                │   │  │
│  │  └──────────────┬───────────────────────┘   │  │
│  │  ┌──────────────▼───────────────────────┐   │  │
│  │  │   Hybrid Search Engine               │   │  │
│  │  │  - Vector Similarity (HNSW)          │   │  │
│  │  │  - BM25 Keyword Search               │   │  │
│  │  │  - Reranker                          │   │  │
│  │  │  - Query Expansion                   │   │  │
│  │  └──────────────┬───────────────────────┘   │  │
│  │  ┌──────────────▼───────────────────────┐   │  │
│  │  │   Memory Consolidation               │   │  │
│  │  │  - Similarity Detection               │   │  │
│  │  │  - Conflict Resolution                │   │  │
│  │  │  - Merge Strategy                     │   │  │
│  │  └──────────────────────────────────────┘   │  │
│  └─────────────────────────────────────────────┘  │
│                                                    │
│  ┌─────────────────────────────────────────────┐  │
│  │          SahomeDB Storage Layer             │  │
│  │  ┌────────────────────────────────────────┐ │  │
│  │  │  Global: ~/.config/rag-mcp/global.db   │ │  │
│  │  │  Projects: <project>/.rag-mcp/data.db  │ │  │
│  │  │  Format: Sled + HNSW Index             │ │  │
│  │  └────────────────────────────────────────┘ │  │
│  └─────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Data Model

### Memory Record Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,              // UUID v4
    pub content: String,         // Original text
    pub embedding: Vec<f32>,     // 768-dim vector
    pub metadata: MemoryMetadata,
    pub scope: MemoryScope,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryScope {
    Session,                     // In-memory only
    Project { path: PathBuf },   // Per-project database
    Global,                      // Shared database
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
```

## Chunking Strategy

### Semantic AST-Aware Chunker
```rust
pub struct SemanticChunker {
    max_chunk_size: usize,      // ~512 tokens
    min_chunk_size: usize,      // ~128 tokens
    overlap: usize,             // ~50 tokens
    language: Language,
}

impl SemanticChunker {
    // Priority-based splitting at semantic boundaries:
    // 1. Top-level declarations (fn, struct, impl, mod)
    // 2. Block boundaries (function bodies, if/match blocks)
    // 3. Statement boundaries
    // 4. Paragraph boundaries (for docs/comments)
    // 5. Sentence boundaries
    // 6. Token-based split (fallback)
    
    pub fn chunk(&self, code: &str) -> Vec<Chunk> {
        let tree = self.parse_tree(code);
        let mut chunks = Vec::new();
        
        // Walk AST, identify semantic boundaries
        for node in tree.root_node().children() {
            if self.is_semantic_boundary(&node) {
                let chunk = self.extract_chunk(&node, code);
                if chunk.len() > self.max_chunk_size {
                    // Recursively split large nodes
                    chunks.extend(self.split_large_node(&node, code));
                } else {
                    chunks.push(chunk);
                }
            }
        }
        
        // Add overlap for context preservation
        self.add_overlap(&mut chunks);
        chunks
    }
}
```

### Supported Languages (Tree-sitter grammars)
- Rust: `tree-sitter-rust`
- JavaScript/TypeScript: `tree-sitter-javascript`, `tree-sitter-typescript`
- Python: `tree-sitter-python`
- Go: `tree-sitter-go`
- C/C++: `tree-sitter-c`, `tree-sitter-cpp`
- Java: `tree-sitter-java`
- Markdown: `tree-sitter-md`

## Embedding Pipeline

### BERT Embedder
```rust
pub struct BertEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl BertEmbedder {
    pub fn new() -> Result<Self> {
        // Load all-mpnet-base-v2 from HuggingFace
        let model = BertModel::from_pretrained("sentence-transformers/all-mpnet-base-v2")?;
        let tokenizer = Tokenizer::from_pretrained("sentence-transformers/all-mpnet-base-v2")?;
        
        Ok(Self {
            model,
            tokenizer,
            device: Device::cuda_if_available(),
        })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text, true)?;
        let input_ids = Tensor::new(tokens.get_ids(), &self.device)?;
        let attention_mask = Tensor::new(tokens.get_attention_mask(), &self.device)?;
        
        let output = self.model.forward(&input_ids, &attention_mask)?;
        let embeddings = self.mean_pooling(&output, &attention_mask)?;
        
        Ok(embeddings.to_vec1()?)
    }
}
```

## Hybrid Search Engine

### Search Implementation
```rust
pub struct HybridSearchEngine {
    vector_index: Collection,    // SahomeDB HNSW
    bm25: BM25Index,             // Custom BM25 impl
    reranker: Option<Reranker>,
}

impl HybridSearchEngine {
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        // 1. Vector similarity search
        let query_embedding = self.embedder.embed(query)?;
        let vector_results = self.vector_index.search(&query_embedding, k * 2)?;
        
        // 2. BM25 keyword search
        let bm25_results = self.bm25.search(query, k * 2)?;
        
        // 3. Reciprocal Rank Fusion (RRF)
        let mut fused = self.fuse_results(vector_results, bm25_results, k);
        
        // 4. Optional reranking
        if let Some(reranker) = &self.reranker {
            fused = reranker.rerank(query, fused)?;
        }
        
        Ok(fused.into_iter().take(k).collect())
    }
    
    fn fuse_results(&self, 
        vector: Vec<(String, f32)>,
        bm25: Vec<(String, f32)>,
        k: usize
    ) -> Vec<SearchResult> {
        // RRF: score = 1/(k + rank)
        // Combined score from both rankings
        let k_const = 60.0;
        // Implementation details...
    }
}
```

### Query Expansion
```rust
pub struct QueryExpander {
    // Generate multiple query variations
    pub fn expand(&self, query: &str) -> Vec<String> {
        vec![
            query.to_string(),
            self.add_synonyms(query),
            self.reformulate(query),
        ]
    }
}
```

## Memory Consolidation

### Consolidation Strategy
```rust
pub struct MemoryConsolidator {
    similarity_threshold: f32,  // 0.85 default
}

impl MemoryConsolidator {
    pub async fn consolidate(&self, memories: &[Memory]) -> Result<Vec<Memory>> {
        // 1. Cluster similar memories
        let clusters = self.cluster_by_similarity(memories)?;
        
        // 2. For each cluster, merge similar memories
        let mut consolidated = Vec::new();
        for cluster in clusters {
            if cluster.len() > 1 {
                let merged = self.merge_cluster(cluster)?;
                consolidated.push(merged);
            } else {
                consolidated.extend(cluster);
            }
        }
        
        Ok(consolidated)
    }
    
    fn merge_cluster(&self, cluster: Vec<Memory>) -> Result<Memory> {
        // Conflict resolution:
        // 1. Keep most recent content
        // 2. Merge tags (union)
        // 3. Average importance scores
        // 4. Preserve all source references
        // Implementation...
    }
}
```

## HNSW Configuration (SahomeDB)

```rust
pub struct HnswConfig {
    pub m: usize,                    // 32 (connections per node)
    pub ef_construction: usize,      // 200 (build quality)
    pub ef_search: usize,            // 50 (search quality, runtime)
    pub distance: Distance,          // Cosine similarity
}

// SahomeDB usage
let mut config = Config::default();
config.distance = Distance::Cosine;
config.m = 32;
config.ef_construction = 200;

let collection = Collection::create(config)?;
```

## MCP Protocol Implementation

### Tool Definitions

#### 1. store_memory
```json
{
  "name": "store_memory",
  "description": "Store new memory with semantic embeddings",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": {"type": "string"},
      "scope": {
        "type": "string",
        "enum": ["session", "project", "global"]
      },
      "metadata": {
        "type": "object",
        "properties": {
          "tags": {"type": "array", "items": {"type": "string"}},
          "source_file": {"type": "string"},
          "language": {"type": "string"}
        }
      }
    },
    "required": ["content", "scope"]
  }
}
```

#### 2. search_memory
```json
{
  "name": "search_memory",
  "description": "Semantic + keyword hybrid search",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {"type": "string"},
      "k": {"type": "integer", "default": 5},
      "scope": {"type": "string"},
      "filters": {
        "type": "object",
        "properties": {
          "tags": {"type": "array"},
          "language": {"type": "string"}
        }
      },
      "min_similarity": {"type": "number", "default": 0.7}
    },
    "required": ["query"]
  }
}
```

#### 3. update_memory
```json
{
  "name": "update_memory",
  "description": "Update existing memory by ID",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"},
      "content": {"type": "string"},
      "metadata": {"type": "object"}
    },
    "required": ["id"]
  }
}
```

#### 4. delete_memory
```json
{
  "name": "delete_memory",
  "description": "Delete memory by ID",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"}
    },
    "required": ["id"]
  }
}
```

#### 5. list_memories
```json
{
  "name": "list_memories",
  "description": "List/browse memories with pagination",
  "inputSchema": {
    "type": "object",
    "properties": {
      "scope": {"type": "string"},
      "filters": {"type": "object"},
      "limit": {"type": "integer", "default": 50},
      "offset": {"type": "integer", "default": 0}
    }
  }
}
```

#### 6. get_memory
```json
{
  "name": "get_memory",
  "description": "Retrieve memory by ID",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"}
    },
    "required": ["id"]
  }
}
```

#### 7. consolidate_memories
```json
{
  "name": "consolidate_memories",
  "description": "Merge similar memories",
  "inputSchema": {
    "type": "object",
    "properties": {
      "scope": {"type": "string"},
      "threshold": {"type": "number", "default": 0.85}
    }
  }
}
```

#### 8. clear_session
```json
{
  "name": "clear_session",
  "description": "Clear session memory",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### Resource Definitions

#### memory://list
```json
{
  "uri": "memory://list",
  "name": "Memory List",
  "description": "Browse all memories",
  "mimeType": "application/json"
}
```

#### memory://stats
```json
{
  "uri": "memory://stats",
  "name": "Memory Statistics",
  "description": "Database statistics and health",
  "mimeType": "application/json"
}
```

## Configuration System

### Config File: `~/.config/rag-mcp/config.toml`

```toml
[server]
log_level = "info"

[embedding]
model = "sentence-transformers/all-mpnet-base-v2"
dimension = 768
batch_size = 32
device = "cuda"  # or "cpu"

[chunking]
max_chunk_size = 512
min_chunk_size = 128
overlap = 50
strategy = "semantic_ast"

[search]
default_k = 5
min_similarity = 0.7
enable_reranking = false
query_expansion = true

[hnsw]
m = 32
ef_construction = 200
ef_search = 50
distance = "cosine"

[consolidation]
similarity_threshold = 0.85
auto_consolidate = false

[storage]
global_db_path = "~/.config/rag-mcp/global.db"
project_db_name = ".rag-mcp/data.db"
max_session_memories = 1000

[languages]
enabled = ["rust", "javascript", "typescript", "python", "go", "markdown"]
```

### Runtime Reconfiguration
```rust
// Watch config file for changes
pub struct ConfigWatcher {
    watcher: notify::RecommendedWatcher,
    config: Arc<RwLock<Config>>,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let (tx, rx) = channel();
        let watcher = notify::watcher(tx, Duration::from_secs(2))?;
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
        
        // Spawn reload task
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                if let Modify(_) = event {
                    // Reload config
                }
            }
        });
        
        Ok(Self { watcher, config })
    }
}
```

## CLI Interface

```bash
# Server mode (for MCP)
rag-mcp serve

# Manual memory management
rag-mcp add --content "..." --scope project --tags "rust,async"
rag-mcp search "async programming" --k 10
rag-mcp list --scope global --limit 50
rag-mcp delete <id>
rag-mcp update <id> --content "..."

# Index maintenance
rag-mcp consolidate --scope project --threshold 0.85
rag-mcp optimize --scope global
rag-mcp vacuum  # Clean up deleted records

# Import/Export
rag-mcp export --scope project --output backup.json
rag-mcp import --input backup.json --scope project

# Health checks
rag-mcp status
rag-mcp stats --scope global
```

## Zed Extension Structure

### Directory Layout
```
zed-rag-mcp/
├── extension.toml
├── Cargo.toml
├── src/
│   └── lib.rs
└── README.md
```

### extension.toml
```toml
id = "rag-mcp"
name = "RAG Memory MCP Server"
description = "Semantic memory and RAG for code documentation"
version = "0.1.0"
schema_version = 1
authors = ["Your Name <email@example.com>"]
repository = "https://github.com/user/zed-rag-mcp"

[context_servers.rag-memory]
command = "rag-mcp"
args = ["serve"]

[[context_servers.rag-memory.settings]]
name = "global_db_path"
description = "Path to global memory database"
default = "~/.config/rag-mcp/global.db"

[[context_servers.rag-memory.settings]]
name = "embedding_model"
description = "BERT model for embeddings"
default = "sentence-transformers/all-mpnet-base-v2"
```

### src/lib.rs (Extension Wrapper)
```rust
use zed_extension_api::{self as zed, Result};

struct RagMcpExtension;

impl zed::Extension for RagMcpExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        context_server_id: &zed::ContextServerId,
        project: &zed::Project,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.get_binary_path()?,
            args: vec!["serve".into()],
            env: vec![
                ("RAG_PROJECT_PATH".into(), project.worktree().to_string_lossy().into()),
                ("RAG_LOG_LEVEL".into(), "info".into()),
            ],
        })
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Err("Not a language server".into())
    }
}

zed::register_extension!(RagMcpExtension);
```

## Distribution Strategy

### GitHub Releases
Build for all platforms:
- macOS ARM64: `aarch64-apple-darwin`
- macOS x86_64: `x86_64-apple-darwin`
- Linux x86_64: `x86_64-unknown-linux-gnu`
- Linux ARM64: `aarch64-unknown-linux-gnu`
- Windows x86_64: `x86_64-pc-windows-msvc`

### extension.toml (Binary Distribution)
```toml
[context_servers.rag-memory.targets.darwin-aarch64]
archive = "https://github.com/user/rag-mcp/releases/download/v0.1.0/rag-mcp-darwin-arm64.tar.gz"
sha256 = "abc123..."

[context_servers.rag-memory.targets.darwin-x86_64]
archive = "https://github.com/user/rag-mcp/releases/download/v0.1.0/rag-mcp-darwin-x64.tar.gz"
sha256 = "def456..."

[context_servers.rag-memory.targets.linux-x86_64]
archive = "https://github.com/user/rag-mcp/releases/download/v0.1.0/rag-mcp-linux-x64.tar.gz"
sha256 = "ghi789..."
```

## Performance Targets

- **Embedding**: < 50ms per chunk (768-dim, GPU)
- **Search**: < 100ms for top-5 results (100K memories)
- **Indexing**: < 200ms per document (average 500 tokens)
- **Memory footprint**: < 500MB (excluding model weights)
- **Startup time**: < 2s (model loading)

## Usage in Claude Code

### Example Prompts

**Storing Documentation:**
```
Store this Rust async documentation in project memory:
[paste documentation]
Tag it with: rust, async, tokio
```

**Searching Memory:**
```
Search my project memory for "error handling patterns in async Rust"
```

**Code Understanding:**
```
Using my project memories, explain how our authentication system works
```

### README Section for Users
```markdown
## Using RAG Memory in Zed/Claude Code

The RAG MCP server automatically stores and retrieves relevant documentation as you work. 

### Storing Memories
Ask Claude to store important information:
- "Remember that our API uses JWT tokens for auth"
- "Store this error handling pattern for future reference"
- "Save these TypeScript type definitions"

### Searching Memories
Claude will automatically search your memories when relevant, but you can also ask explicitly:
- "What do I know about database migrations?"
- "Find my notes on React hooks"
- "Search for async patterns in my project"

### Scopes
- **Session**: Temporary memories (current session only)
- **Project**: Stored in `.rag-mcp/` (per-project)
- **Global**: Shared across all projects

### Configuration
Edit `~/.config/rag-mcp/config.toml` to customize behavior.
Changes apply immediately without restart.
```

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)
- [ ] Project setup with Cargo workspace
- [ ] SahomeDB integration
- [ ] BERT embedding pipeline
- [ ] Basic MCP server with store/search tools
- [ ] Configuration system

### Phase 2: Chunking & AST (Week 3)
- [ ] Tree-sitter integration
- [ ] Semantic AST-aware chunker
- [ ] Multi-language support
- [ ] Metadata extraction

### Phase 3: Search Engine (Week 4)
- [ ] BM25 keyword search
- [ ] Hybrid search with RRF
- [ ] Query expansion
- [ ] Result filtering

### Phase 4: Memory Management (Week 5)
- [ ] Consolidation algorithm
- [ ] Conflict resolution
- [ ] Session/Project/Global scopes
- [ ] CLI tools

### Phase 5: Zed Integration (Week 6)
- [ ] Zed extension wrapper
- [ ] Configuration UI
- [ ] Testing with Claude Code
- [ ] Documentation

### Phase 6: Polish & Release (Week 7-8)
- [ ] Performance optimization
- [ ] Cross-platform builds
- [ ] CI/CD pipeline
- [ ] Release to Zed extension marketplace

## Testing Strategy

### Unit Tests
- Chunking algorithm correctness
- Embedding consistency
- Search ranking quality
- Consolidation logic

### Integration Tests
- Full store → search → retrieve flow
- MCP protocol compliance
- Multi-scope operations
- Config hot-reloading

### Benchmark Suite
- Embedding throughput
- Search latency (10K, 100K records)
- Memory usage under load
- Startup time

## Security Considerations

1. **Local-only**: All data stored locally, no network calls
2. **File permissions**: Restrict database files to user-only
3. **Path validation**: Sanitize all file paths
4. **Resource limits**: Prevent OOM with max memory limits
5. **Input validation**: Sanitize all user inputs

## Future Enhancements

1. **Advanced features**:
   - Automatic memory decay/forgetting
   - Importance scoring with ML
   - Graph relationships between memories
   - Multi-modal support (images, diagrams)

2. **Performance**:
   - Quantized embeddings (reduced memory)
   - Disk-based HNSW for larger datasets
   - Parallel batch processing

3. **Integration**:
   - Git integration (memory per branch)
   - LSP integration for live code context
   - File watcher for auto-indexing

---

## Quick Start Commands

```bash
# Build release binary
cargo build --release

# Install locally
cargo install --path .

# Run server (for testing)
rag-mcp serve

# Add test memory
rag-mcp add --content "Rust async programming example" --scope global --tags rust,async

# Search
rag-mcp search "async patterns" --k 5

# Install Zed extension (dev mode)
# In Zed: cmd-shift-p > "zed: install dev extension"
# Select the extension directory
```

This specification provides a complete blueprint for building a production-grade RAG system for Zed/Claude Code!