# Total Recall - RAG MCP Server

A production-ready RAG (Retrieval-Augmented Generation) system written in Rust, designed as an MCP (Model Context Protocol) server for Zed editor and Claude Code.

## Features

- **BM25 Keyword Search**: Fast, proven probabilistic ranking for information retrieval
- **Multi-Scope Memory**: Session (in-memory), Project (per-project DB), and Global (shared DB)
- **MCP Protocol**: Full JSON-RPC 2.0 implementation over stdio
- **Embedded Storage**: Sled database, no external dependencies
- **CLI Tools**: Complete command-line interface for memory management
- **Zero Dependencies**: No Python, no ML models, pure Rust

## Status

✅ **Phase 1 Complete** - BM25 search and MCP server fully functional

## Quick Start

### Build

```bash
cargo build --release
```

### CLI Usage

```bash
# Add a memory
./target/release/rag-mcp add --content "Your content here" --tags rust --tags async

# Search memories
./target/release/rag-mcp search "search query" --k 5

# List all memories
./target/release/rag-mcp list --limit 10

# Show statistics
./target/release/rag-mcp stats

# Run as MCP server (for Zed/Claude Code)
./target/release/rag-mcp serve
```

### Memory Scopes

- **session**: Temporary, in-memory (cleared on exit)
- **project**: Stored in `<project>/.rag-mcp/data.db`
- **global**: Shared across all projects at `~/.config/rag-mcp/global.db`

## Project Structure

```
totalrecall/
├── crates/
│   ├── rag-mcp-server/   # MCP server binary and CLI
│   ├── rag-core/         # Core data structures, storage, config
│   └── rag-search/       # BM25 search engine
├── SPEC.md               # Technical specification
├── PROG.md               # Programming rules
└── CLAUDE.md             # AI development guidelines
```

## MCP Tools

When running as MCP server, provides these tools:

- `store_memory`: Store new memory with tags and scope
- `search_memory`: BM25 keyword search
- `list_memories`: Browse memories with pagination
- `delete_memory`: Delete by ID
- `clear_session`: Clear session memories

## Configuration

Edit `~/.config/rag-mcp/config.toml` to customize:

```toml
[server]
log_level = "info"

[search]
default_k = 5
bm25_k1 = 1.2
bm25_b = 0.75

[storage]
global_db_path = "~/.config/rag-mcp/global.db"
max_session_memories = 1000
```

## License

MIT OR Apache-2.0
