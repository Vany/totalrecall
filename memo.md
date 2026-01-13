# Development Memo

## 2026-01-13: Initial Setup

### Repository Initialized
- Created git repository with main branch
- Set up Cargo workspace with 5 crates
- All crates compile successfully

### Project Structure
```
totalrecall/
├── crates/
│   ├── rag-mcp-server/   # Binary crate - MCP server + CLI
│   ├── rag-core/         # Library - Core data structures
│   ├── rag-embedding/    # Library - BERT embeddings
│   ├── rag-chunking/     # Library - AST-aware chunking
│   └── rag-search/       # Library - Hybrid search engine
```

### Dependencies
- **Storage**: sahomedb v0.4.0
- **Embeddings**: candle-core, candle-nn, candle-transformers
- **AST Parsing**: tree-sitter + 9 language grammars
- **Async**: tokio v1.42
- **CLI**: clap v4.5

### Known Issues
1. **Python Version Compatibility**: tokenizers crate requires PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 flag for Python 3.14
   - Solution: Use build.sh script or set environment variable
   - Alternative: Upgrade to tokenizers v0.22+ in future

2. **Warnings**: Minor dead_code warnings in stubs - expected, will disappear during implementation

### Build Instructions
```bash
# Using helper script (recommended)
./build.sh

# Or manually
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
cargo build --release
```

### Next Steps (Phase 1)
- [ ] Implement core storage layer with SahomeDB
- [ ] Set up configuration system
- [ ] Implement basic MCP protocol handlers
- [ ] Add store_memory and search_memory tools

### Git History
- `3dd2a50` - Initial commit with complete workspace structure
