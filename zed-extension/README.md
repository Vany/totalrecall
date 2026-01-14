# Total Recall - Zed Extension

A memory system with BM25 search for Zed editor, enabling AI assistants to store and retrieve context across coding sessions.

## Features

- **Store Memories** - Save code snippets, documentation, and project knowledge
- **BM25 Search** - Fast keyword search with relevance ranking  
- **Multi-Scope Storage** 
  - **Session**: Temporary memories (cleared when Zed closes)
  - **Project**: Stored in `.rag-mcp/` within your project
  - **Global**: Shared across all projects
- **Zero Configuration** - Works out of the box
- **Cross-Platform** - macOS (Apple Silicon), Linux, Windows

## Installation

1. Open Zed
2. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux/Windows)
3. Type "extensions" and select "zed: extensions"
4. Search for "Total Recall"
5. Click Install

The extension will automatically download the appropriate MCP server binary for your platform on first use.

## Usage

Once installed, Total Recall adds memory capabilities to Claude and other AI assistants in Zed.

### Store Information

Ask the AI to remember important information:

```
Remember that our API uses JWT tokens with 24-hour expiration.
Store this for future reference.
```

The AI will use the `store_memory` tool to save this information.

### Search Memories

Ask the AI to recall information:

```
What do you know about our authentication system?
```

The AI will automatically search your memories using the `search_memory` tool.

### Manage Memories

List all stored memories:
```
Show me all my project memories
```

Clear session memories:
```
Clear my session memories
```

## Memory Scopes

### Session Scope
- Temporary, in-memory only
- Cleared when Zed closes
- Perfect for temporary context during a coding session

### Project Scope  
- Stored in `<project>/.rag-mcp/data.db`
- Persists across sessions
- Specific to each project
- Shared with team if committed to git

### Global Scope
- Stored in `~/Library/Application Support/rag-mcp/global.db` (macOS)
- Shared across all projects
- Personal knowledge base
- Persists indefinitely

## MCP Tools

The extension provides these Model Context Protocol tools:

- `store_memory` - Store content with tags and scope
- `search_memory` - BM25 keyword search across memories
- `list_memories` - Browse stored memories with pagination
- `delete_memory` - Remove specific memory by ID
- `clear_session` - Clear all session-scoped memories

## Configuration

Total Recall works without configuration, but you can customize it by editing `~/.config/rag-mcp/config.toml`:

```toml
[server]
log_level = "info"

[search]
default_k = 5        # Number of search results
bm25_k1 = 1.2        # BM25 term frequency saturation
bm25_b = 0.75        # BM25 length normalization

[storage]
global_db_path = "~/.config/rag-mcp/global.db"
```

## Troubleshooting

### Extension Not Loading

Check Zed's developer console for errors:
- View → Toggle Developer Tools
- Look for "totalrecall" errors

### Binary Download Issues

The extension downloads the MCP server binary on first use. If this fails:

1. Check your internet connection
2. Verify GitHub is accessible
3. Check Zed logs for download errors

### Manual Installation

If automatic download fails, you can install the binary manually:

```bash
# Install from source
cargo install --git https://github.com/Vany/totalrecall

# Or download from releases
# https://github.com/Vany/totalrecall/releases
```

Then configure the extension to use your binary (advanced users only).

## Privacy & Security

- **Local-only**: All data stored locally on your machine
- **No network calls**: Except for downloading the binary on first install
- **Your control**: You can delete memories at any time
- **SQLite storage**: Standard, inspectable database format

## Development

See the main repository for development instructions:  
https://github.com/Vany/totalrecall

## License

MIT - See LICENSE file

## Support

- Issues: https://github.com/Vany/totalrecall/issues
- Repository: https://github.com/Vany/totalrecall
