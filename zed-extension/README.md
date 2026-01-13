# Total Recall - Zed Extension

Semantic memory system for Zed editor using BM25 keyword search.

## Features

- **Store memories** - Save important code snippets, documentation, and notes
- **BM25 search** - Fast keyword search with relevance ranking
- **Multi-scope** - Session (temporary), Project (local), Global (shared)
- **Zero dependencies** - Pure Rust, no Python or ML models required

## Installation

### Option 1: Install from Binary (Recommended)

1. Build or download the `rag-mcp` binary:
   ```bash
   cargo install --git https://github.com/Vany/totalrecall
   ```

2. Install this extension in Zed:
   - Open Zed
   - Press `cmd-shift-p` (macOS) or `ctrl-shift-p` (Linux/Windows)
   - Type "zed: extensions"
   - Click "Install Dev Extension"
   - Select this directory

### Option 2: Build from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/Vany/totalrecall.git
   cd totalrecall
   ```

2. Build the binary:
   ```bash
   cargo build --release
   ```

3. Copy to your PATH:
   ```bash
   cp target/release/rag-mcp ~/.local/bin/  # or /usr/local/bin
   ```

4. Install the Zed extension (same as Option 1, step 2)

## Usage

Once installed, Total Recall adds these tools to Claude/AI assistants in Zed:

### Store Memory
```
Store this information about Rust async:
- async/await is built on futures
- tokio is the most popular runtime
```

The AI will use the `store_memory` tool to save this.

### Search Memory
```
What do I know about async Rust?
```

The AI will use the `search_memory` tool to find relevant memories.

### List Memories
```
Show me all my memories about databases
```

### Clear Session
```
Clear my session memories
```

## Memory Scopes

- **session** - Temporary, cleared when Zed closes
- **project** - Stored in `.rag-mcp/` in your project
- **global** - Shared across all projects at `~/.config/rag-mcp/`

## Configuration

Edit `~/.config/rag-mcp/config.toml`:

```toml
[server]
log_level = "info"

[search]
default_k = 5
bm25_k1 = 1.2
bm25_b = 0.75

[storage]
global_db_path = "~/.config/rag-mcp/global.db"
```

## Troubleshooting

### Binary not found
Make sure `rag-mcp` is in your PATH:
```bash
which rag-mcp
```

### Check logs
View MCP server logs in Zed's developer tools.

### Test manually
Test the server works:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | rag-mcp serve
```

## Development

See the main repository for development instructions:
https://github.com/Vany/totalrecall
