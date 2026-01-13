# Testing Total Recall with Zed

## Quick Start Guide

### Step 1: Install the Binary

First, make sure the `rag-mcp` binary is available in your PATH:

```bash
# From the totalrecall directory
cargo install --path crates/rag-mcp-server

# Or copy the release binary
cp target/release/rag-mcp ~/.local/bin/

# Verify it's in PATH
which rag-mcp
```

### Step 2: Install the Zed Extension (Dev Mode)

1. Open Zed editor

2. Open the command palette:
   - macOS: `cmd + shift + p`
   - Linux/Windows: `ctrl + shift + p`

3. Type and select: **"zed: install dev extension"**

4. Navigate to and select the `zed-extension` directory in this repo

5. The extension will be installed and Total Recall MCP server will start automatically

### Step 3: Verify Installation

1. Open Zed's developer console:
   - Menu: `View > Developer Tools`
   - Or: `cmd + option + i` (macOS)

2. Look for MCP server initialization logs

3. You should see Total Recall listed in the MCP servers section

### Step 4: Test with Claude

Now you can use Total Recall with Claude in Zed! Try these prompts:

#### Store a Memory
```
Remember this: Rust's ownership system prevents data races at compile time.
Tag it with: rust, memory-safety
```

Claude will use the `store_memory` tool.

#### Search Memories
```
What do I know about Rust?
```

Claude will use the `search_memory` tool to find relevant memories.

#### List Memories
```
List all my memories
```

#### Clear Session
```
Clear my session memories
```

## Manual Testing (CLI)

You can also test the MCP server directly without Zed:

```bash
# Test initialization
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | rag-mcp serve

# Test tools list
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | rag-mcp serve

# Test store memory
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"store_memory","arguments":{"content":"Test memory","scope":"session","tags":["test"]}}}' | rag-mcp serve
```

## Configuration

The MCP server reads configuration from:
- `~/.config/rag-mcp/config.toml` (auto-created on first run)

You can customize:
- Log level
- Search parameters (BM25 k1, b)
- Database paths
- Default result count

## Troubleshooting

### Server Not Starting

1. Check the binary is in PATH:
   ```bash
   which rag-mcp
   ```

2. Test the binary manually:
   ```bash
   rag-mcp --version
   ```

3. Check Zed's console for error messages

### No Tools Appearing

1. Restart Zed after installing the extension

2. Check the extension is listed in Zed's extensions panel

3. Verify logs show successful server initialization

### Connection Issues

1. Check stderr output in Zed console

2. Run the server manually to see errors:
   ```bash
   rag-mcp serve
   # Then type a JSON-RPC request
   ```

## What's Happening Under the Hood

1. **Zed starts the MCP server** when you open a project
   - Runs: `rag-mcp serve`
   - Communicates via stdio (JSON-RPC 2.0)

2. **Claude discovers tools** via MCP protocol
   - Calls `initialize` to handshake
   - Calls `tools/list` to get available tools
   - Tools appear in Claude's context

3. **Claude uses tools** when responding to you
   - Calls `tools/call` with tool name and arguments
   - Server processes the request
   - Returns results to Claude
   - Claude incorporates results in response

## Next Steps

Once you've tested locally:

1. Report any issues on GitHub
2. Suggest improvements
3. Consider publishing to Zed extension marketplace

## Reference

- **MCP Protocol**: https://modelcontextprotocol.io
- **Zed Extensions**: https://zed.dev/docs/extensions
- **Repository**: https://github.com/Vany/totalrecall
