# Total Recall - RAG MCP Server

A production-grade Retrieval-Augmented Generation (RAG) system written in Rust, designed as an MCP (Model Context Protocol) server for Zed editor and Claude Code.

## Features

- **Semantic Memory**: Store and retrieve code documentation with vector embeddings
- **AST-Aware Chunking**: Intelligent code splitting using Tree-sitter
- **Hybrid Search**: Combines vector similarity (HNSW) and BM25 keyword search
- **Multi-Scope**: Session, Project, and Global memory scopes
- **Production-Ready**: Built with Rust for performance and reliability

## Project Structure

```
totalrecall/
├── crates/
│   ├── rag-mcp-server/   # MCP server binary and CLI
│   ├── rag-core/         # Core data structures
│   ├── rag-embedding/    # BERT embeddings
│   ├── rag-chunking/     # AST-aware semantic chunking
│   └── rag-search/       # Hybrid search engine
├── SPEC.md               # Technical specification
├── PROG.md               # Programming rules
└── CLAUDE.md             # AI development guidelines
```

## Status

🚧 **Under Development** - Initial boilerplate created

See `SPEC.md` for complete technical specification and implementation plan.

## Build

```bash
cargo build --release
```

## License

MIT OR Apache-2.0
