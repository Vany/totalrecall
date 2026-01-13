# Total Recall - Claude Code Integration

Total Recall works with Claude Code (the AI coding assistant) using MCP!

## Installation Steps

### Step 1: Install the Binary

```bash
# From the totalrecall directory
cargo install --path crates/rag-mcp-server

# Verify installation
which rag-mcp
rag-mcp --help
```

### Step 2: Configure Claude Code

Claude Code uses MCP servers defined in your user settings. You have two options:

#### Option A: Manual Configuration (Recommended)

1. Open Claude Code settings
   - macOS: `~/.config/claude/claude_desktop_config.json`
   - Linux: `~/.config/claude/claude_desktop_config.json`
   - Windows: `%APPDATA%\Claude\claude_desktop_config.json`

2. Add Total Recall to the MCP servers section:

```json
{
  "mcpServers": {
    "totalrecall": {
      "command": "rag-mcp",
      "args": ["serve"]
    }
  }
}
```

3. Restart Claude Code

#### Option B: Using Zed Extension (if in Zed)

If you're using Claude Code within Zed:

1. Open Zed
2. `Cmd+Shift+P` → "zed: install dev extension"
3. Select: `/Users/vany/l/totalrecall/zed-extension`
4. Extension auto-configures Claude Code

### Step 3: Verify Installation

1. Start a new conversation in Claude Code

2. Check if Total Recall tools are available:
   - Look for tool indicators in the UI
   - Or ask: "What tools do you have access to?"

3. You should see these tools:
   - `store_memory`
   - `search_memory`
   - `list_memories`
   - `delete_memory`
   - `clear_session`

## Usage Examples

### Store Information
```
Remember this: Rust's ownership system prevents data races at compile time.
Tag it with: rust, memory-safety
```

Claude will use the `store_memory` tool.

### Search Memories
```
What do I know about Rust?
```

Claude will search using BM25 and return relevant memories.

### List All Memories
```
Show me all my stored memories
```

### Search for Specific Topics
```
Find everything about databases
```

## Memory Scopes

Total Recall supports three scopes:

1. **Session** - Temporary, cleared when Claude Code closes
2. **Project** - Stored in your project directory (`.rag-mcp/`)
3. **Global** - Shared across all projects (`~/.config/rag-mcp/`)

By default, memories are stored globally unless you specify otherwise.

## Configuration

Customize behavior in `~/.config/rag-mcp/config.toml`:

```toml
[server]
log_level = "info"

[search]
default_k = 5          # Number of search results
bm25_k1 = 1.2          # BM25 parameter
bm25_b = 0.75          # BM25 parameter

[storage]
global_db_path = "~/.config/rag-mcp/global.db"
max_session_memories = 1000
```

## Troubleshooting

### Tools Not Appearing

1. **Check binary is in PATH:**
   ```bash
   which rag-mcp
   ```

2. **Test MCP server manually:**
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | rag-mcp serve
   ```
   Should return JSON with capabilities.

3. **Check configuration file:**
   ```bash
   cat ~/.config/claude/claude_desktop_config.json
   ```
   Verify "totalrecall" is listed in mcpServers.

4. **Restart Claude Code** after configuration changes

### Server Errors

Check logs in Claude Code's developer console or run manually:
```bash
rag-mcp serve 2>&1 | tee /tmp/rag-mcp.log
```

### Permission Issues

Ensure the binary is executable:
```bash
chmod +x $(which rag-mcp)
```

## Advanced Usage

### Multiple Projects

Each project can have its own memories:
```
Store this in project scope: <information>
```

### Tagging Strategy

Use consistent tags for better organization:
- Programming languages: `rust`, `python`, `javascript`
- Topics: `async`, `database`, `testing`
- Importance: `important`, `reference`, `snippet`

### Searching Tips

- Use specific keywords for better BM25 ranking
- Combine multiple terms: "rust async error handling"
- Ask Claude to search before answering questions

## Example Workflow

```
You: Remember: Our API uses JWT tokens with 1-hour expiration. 
     Refresh tokens last 7 days. Tag: auth, api, jwt

Claude: I'll store that memory about your API authentication...
        [Uses store_memory tool]

--- Later ---

You: How does our authentication work?

Claude: Let me search my memories...
        [Uses search_memory tool]
        Based on what I have stored: Your API uses JWT tokens...
```

## Performance

- **Storage**: Instant (Sled database)
- **Search**: <50ms typical (BM25 algorithm)
- **Memory**: Minimal overhead
- **Startup**: <100ms

## What's Next

Once you've tested and it works:

1. ⭐ Star the repo: https://github.com/Vany/totalrecall
2. Report issues or suggestions
3. Share your use cases!

## Support

- **Documentation**: See README.md
- **Issues**: https://github.com/Vany/totalrecall/issues
- **Tests**: Run `cargo test` to verify functionality

---

**Enjoy your semantic memory with Claude Code!** 🧠✨
