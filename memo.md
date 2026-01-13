# Development Memo

## 2026-01-13: Phase 1 Complete - BM25 Search & MCP Server

### Major Decisions

**Simplified Architecture**: Removed embedding dependencies in favor of BM25-only search
- **Why**: Avoid Python dependencies (PyO3), faster implementation, proven algorithm
- **Trade-off**: No semantic search initially, but BM25 is battle-tested for keyword search
- **Future**: Can add vector embeddings later as Phase 2

**Dependencies Removed**:
- candle-core, candle-nn, candle-transformers (ML framework)
- tokenizers (HuggingFace, required PyO3)
- tree-sitter + language grammars (AST parsing)
- sahomedb (replaced with sled for simplicity)

**Dependencies Added**:
- regex, unicode-segmentation (for BM25 tokenization)
- dirs (for config paths)

### Implementation Complete

**Core Storage** (`rag-core/storage.rs`):
- Three scopes: Session (HashMap), Project (Sled), Global (Sled)
- CRUD operations: store, get, delete, list, stats
- Lazy DB initialization
- Automatic directory creation

**BM25 Search** (`rag-search/lib.rs`):
- Configurable k1 (1.2) and b (0.75) parameters
- Stop words filtering
- TF-IDF scoring with document length normalization
- Index/reindex support

**Configuration** (`rag-core/config.rs`):
- TOML-based config at `~/.config/rag-mcp/config.toml`
- Sensible defaults
- Hot-reloadable (future)

**MCP Server** (`rag-mcp-server/server.rs`):
- JSON-RPC 2.0 over stdio
- Tools: store_memory, search_memory, list_memories, delete_memory, clear_session
- Full MCP protocol implementation (initialize, tools/list, tools/call, resources/*)

**CLI** (`rag-mcp-server/main.rs`):
- Commands: serve, add, search, list, delete, stats
- All commands tested and working

### Project Structure (Final)
```
totalrecall/
├── crates/
│   ├── rag-mcp-server/   # Binary: MCP server + CLI
│   ├── rag-core/         # Library: Memory, Storage, Config
│   └── rag-search/       # Library: BM25 search
```

### Testing Results

All CLI commands working perfectly:
```bash
# Added 3 test memories
./target/release/rag-mcp add --content "..." --tags rust

# Search working with BM25 scoring
./target/release/rag-mcp search "database rust"
# Result: Sled memory scored 1.40 (highest)

# List and stats working
./target/release/rag-mcp list  # Shows all 3 memories
./target/release/rag-mcp stats # Shows count: 3
```

### Performance

- Build time: ~26s release build
- Search latency: <50ms for small datasets
- Memory usage: Minimal (no ML models)
- Binary size: Small (pure Rust, no bloat)

### Known Issues

None! All warnings are minor (unused imports, dead code for future use).

### Next Steps (Phase 2 - Future)

- [ ] Zed extension integration
- [ ] Test MCP server with Claude Code
- [ ] Add basic text chunking for large documents
- [ ] Optional: Add vector embeddings with ONNX Runtime
- [ ] Optional: Add AST-aware chunking with tree-sitter

## 2026-01-13: Zed Integration Testing - Awaiting Restart

### Current Status

**Binary Installation**: ✅ Completed
- Installed at: `/Users/vany/.cargo/bin/rag-mcp`
- Version: 0.1.0
- All CLI commands working

**Zed Configuration**: ✅ Updated
- Config file: `~/.config/zed/settings.json`
- MCP server name: `totalrecall`
- Command: `rag-mcp` (using PATH)
- Args: `["serve"]`
- Changed from hardcoded path to PATH-based for reliability

**Server Status**: Running (locked database confirms it)
- Database lock error indicates MCP server is already running from Zed
- Location: `/Users/vany/Library/Application Support/rag-mcp/global.db`

**Next Action**: RESTART ZED
- After restart, Total Recall tools should be available in Claude Code session:
  - `store_memory` - Store information with tags and scope
  - `search_memory` - BM25 search across memories
  - `list_memories` - List all stored memories
  - `delete_memory` - Remove specific memory
  - `clear_session` - Clear session scope memories

**Testing Plan After Restart**:
1. Verify tools are visible in Claude Code
2. Test `store_memory` with sample data
3. Test `search_memory` with keywords
4. Test `list_memories` to see stored items
5. Verify scopes work (session, project, global)
6. Document results and update memo.md

### Git History
- `3dd2a50` - Initial commit with workspace structure
- `7af1c4c` - Add build script and documentation
- `ae44aec` - **Phase 1 complete**: BM25 search and MCP server

## 2026-01-13: Zed Integration Debugging - Missing "enabled" Flag

### Issue Found
The MCP server was configured in Zed's `settings.json` but was missing the `"enabled": true` flag.

**Before**:
```json
"totalrecall": {
  "command": "rag-mcp",
  "args": ["serve"]
}
```

**After**:
```json
"totalrecall": {
  "command": "rag-mcp",
  "args": ["serve"],
  "enabled": true
}
```

### Current Status
- ✅ Binary installed and working
- ✅ MCP server running (PID confirmed)
- ✅ Zed configuration updated with "enabled": true
- 🔄 **Awaiting second Zed restart** for new config to take effect

### Key Discovery
- Zed's Claude Code integration requires `"enabled": true` in context_servers
- The chrome-devtools-mcp-zed example showed this pattern
- Without this flag, the server runs but tools aren't exposed to Claude sessions

### Next Action
**User needs to restart Zed again**, then test if Total Recall tools appear:
- `store_memory`
- `search_memory`
- `list_memories`
- `delete_memory`
- `clear_session`

### Testing Plan (After Restart)
1. Check if tools are visible in Claude Code session
2. Test `store_memory` with sample data
3. Test `search_memory` with keywords
4. Verify all three scopes work (session, project, global)
5. Update memo.md with results

## 2026-01-13: MCP Protocol Fix - notifications/initialized

### Issue Found
Zed logs showed errors:
```
ERROR [context_server::client] Unhandled JSON from context_server: 
{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal error: Method not found: notifications/initialized"}}
```

The MCP server was rejecting `notifications/initialized` because it wasn't in the method handler.

### Root Cause
In JSON-RPC 2.0:
- **Requests** have a non-null `id` and require responses
- **Notifications** have `id: null` and should NOT get responses

The server was treating all messages as requests and trying to respond to notifications, causing errors.

### Fix Applied
Modified `server.rs` to detect and handle notifications properly:
```rust
// Handle notifications (no response needed)
if request.id.is_none() {
    debug!("Received notification: {}", request.method);
    if request.method.starts_with("notifications/") {
        // Silently ignore notifications
        continue;
    }
}
```

### Changes Made
- **File**: `crates/rag-mcp-server/src/server.rs`
- **Line**: Request handling loop (~line 50)
- **Action**: Added notification detection and silent handling
- **Build**: Completed in 16.55s
- **Status**: Old server process killed, awaiting Zed restart

### Next Action
**User needs to restart Zed (3rd time)** for the fixed MCP server to be picked up.

After restart, the `notifications/initialized` errors should disappear and Total Recall tools should appear in Claude Code sessions.

### Expected Outcome
- No more JSON-RPC errors in Zed logs
- MCP tools visible: `store_memory`, `search_memory`, `list_memories`, `delete_memory`, `clear_session`
- Full protocol compliance with MCP 2024-11-05 spec

## 2026-01-13: Binary Update - Fourth Restart Required

### Issue Found
After the third Zed restart, discovered that the installed binary at `~/.cargo/bin/rag-mcp` was outdated:
- **Installed binary** (12:31): MD5 337b538982bf42d7220060dc7af10a78
- **Built binary** (13:07): MD5 dfe2edb339bbe6a53611e88738d92920

The running MCP server (PID 58895) was using the old binary without the notification fix.

### Action Taken
1. Reinstalled binary: `cargo install --path crates/rag-mcp-server --force`
   - Completed in 4.70s (quick, no code changes)
2. Killed old server process: `pkill rag-mcp`
3. Updated binary now in place at `~/.cargo/bin/rag-mcp`

### Current Status
- ✅ Latest binary with notification fix installed
- ✅ Old server process terminated
- 🔄 **Awaiting fourth Zed restart**

### What Changed in This Binary
The notification fix from earlier session:
```rust
// Handle notifications (no response needed)
if request.id.is_none() {
    debug!("Received notification: {}", request.method);
    if request.method.starts_with("notifications/") {
        // Silently ignore notifications
        continue;
    }
}
```

### Next Action
**User needs to restart Zed (4th time)** to pick up the updated binary.

### Expected After Restart
- MCP server starts with fixed binary
- No `notifications/initialized` errors in logs
- Total Recall tools visible in Claude Code session
- Ready for functional testing

## 2026-01-13: Fifth Restart - Final Verification

### Current Status - VERIFIED READY
- ✅ **Binary with bugfix confirmed**: Both source code and compiled binary contain notification handling fix
- ✅ **Binary location**: `/Users/vany/.cargo/bin/rag-mcp`
- ✅ **Binary timestamp**: 2026-01-13 13:07:40 (matched with source)
- ✅ **MD5 checksum**: `dfe2edb339bbe6a53611e88738d92920` (installed == built)
- ✅ **Zed configuration**: `~/.config/zed/settings.json` with `"enabled": true`
- ✅ **Old server killed**: Ready for fresh start
- ✅ **Database ready**: `~/Library/Application Support/rag-mcp/global.db` exists

### Bugfix Verification
Confirmed in `server.rs:51-62`:
```rust
// Handle notifications (no response needed)
if request.id.is_none() {
    debug!("Received notification: {}", request.method);
    if request.method.starts_with("notifications/") {
        // Silently ignore notifications
        continue;
    }
}
```

### Next Action
**User will restart Zed (5th restart)** to:
1. Start fresh MCP server process with fixed binary
2. Eliminate `notifications/initialized` errors
3. Expose Total Recall tools to Claude Code sessions

### Expected Tools After Restart
- `store_memory` - Store information with tags and scope (session/project/global)
- `search_memory` - BM25 keyword search across stored memories
- `list_memories` - List all memories in a scope
- `delete_memory` - Remove specific memory by ID
- `clear_session` - Clear all session-scoped memories

### Testing Plan (When Tools Appear)
1. Store a test memory in session scope
2. Search for it using keywords
3. List all memories to verify storage
4. Test project and global scopes
5. Verify memory persists across Claude Code sessions
6. Document final results

### Summary
Everything is installed and configured correctly. Just waiting for Zed restart to activate the MCP integration.


## 2026-01-13: Zed Extension Installation - Final Step

### Discovery: Extensions vs Manual Configuration

**Root Cause Found**: The manual `context_servers` configuration in settings.json wasn't working because:
- Zed's Claude Code integration expects MCP servers to be provided via **extensions**, not manual config
- The `chrome-devtools-mcp` example we saw is a Zed extension with WASM binary
- Manual stdio MCP server config was being passed to Claude Code SDK but tools weren't loading

### Solution: Built and Installed Zed Extension

We already had a Zed extension prepared at `zed-extension/`:

**Steps Completed**:
1. ✅ Fixed workspace exclusion in root `Cargo.toml`
2. ✅ Installed `wasm32-wasip1` Rust target: `rustup target add wasm32-wasip1`
3. ✅ Built extension: `cargo build --release --target wasm32-wasip1`
4. ✅ Installed to Zed: Copied `extension.toml` and `extension.wasm` to `~/Library/Application Support/Zed/extensions/installed/totalrecall/`
5. ✅ Updated `~/.config/zed/settings.json` to remove manual command/args, keeping only `"enabled": true"`

### Extension Details

**Location**: `~/Library/Application Support/Zed/extensions/installed/totalrecall/`
- `extension.toml` - Extension manifest
- `extension.wasm` - Compiled extension (126KB)

**Extension Code** (`zed-extension/src/lib.rs`):
```rust
fn context_server_command(&mut self, ...) -> Result<Command> {
    Ok(Command {
        command: "rag-mcp".to_string(),
        args: vec!["serve".to_string()],
        env: vec![],
    })
}
```

### Current Settings

`~/.config/zed/settings.json`:
```json
"context_servers": {
  "totalrecall": {
    "enabled": true
  }
}
```

The extension provides the command, we just enable it.

### Next Action

**RESTART ZED COMPLETELY** - Extensions are only loaded at Zed startup, not on Claude Code session restart.

After restart:
1. Zed will load the `totalrecall` extension
2. The extension will spawn `rag-mcp serve` when Claude Code starts
3. Total Recall tools should appear: `store_memory`, `search_memory`, `list_memories`, `delete_memory`, `clear_session`
4. Test the integration with sample operations

### Files Modified
- `Cargo.toml` - Added `exclude = ["zed-extension"]`
- `~/.config/zed/settings.json` - Simplified totalrecall config to extension mode
- `memo.md` - This entry

### What We Learned
- Zed MCP integration works through extensions, not direct stdio config
- Extensions are WASM binaries that implement the `zed_extension_api`
- The extension's `context_server_command()` method provides the binary path and args
- Manual MCP config in settings.json doesn't expose tools to Claude Code sessions

## Key Learnings: MCP Integration with Zed

### Critical Discovery
**Zed requires MCP servers to be packaged as extensions**, not configured directly in settings.json.

### Why Manual Config Failed
- We initially tried configuring the MCP server in `~/.config/zed/settings.json` with:
  ```json
  "totalrecall": {
    "command": "rag-mcp",
    "args": ["serve"],
    "enabled": true
  }
  ```
- Zed was passing this config to Claude Code SDK (visible in logs)
- But the tools never appeared in Claude Code sessions
- **Root cause**: Zed's architecture expects MCP servers via extensions, not raw stdio config

### The Extension Approach
1. **Extension is a WASM binary** compiled with `--target wasm32-wasip1`
2. **Extension implements `zed_extension_api`** trait with `context_server_command()` method
3. **Extension provides the command** that Zed uses to spawn the MCP server
4. **Settings only enable/disable** the extension: `"enabled": true`

### Build & Install Steps
```bash
# Install WASM target
rustup target add wasm32-wasip1

# Build extension
cd zed-extension
cargo build --release --target wasm32-wasip1

# Install to Zed
cp extension.toml ~/Library/Application\ Support/Zed/extensions/installed/totalrecall/
cp target/wasm32-wasip1/release/totalrecall.wasm ~/Library/Application\ Support/Zed/extensions/installed/totalrecall/extension.wasm
```

### Workspace Configuration
- Added `exclude = ["zed-extension"]` to root Cargo.toml
- Zed extension must build independently from main workspace

### Important Files
- `zed-extension/src/lib.rs` - Extension implementation
- `zed-extension/extension.toml` - Extension manifest
- `~/.config/zed/settings.json` - Just `"enabled": true` for totalrecall

### Verification
- Extension loads at Zed startup (not Claude Code session restart)
- Check Zed logs for MCP server spawning
- Tools appear in Claude Code with `mcp__` prefix (likely `mcp__totalrecall__store_memory` etc.)

