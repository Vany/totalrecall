# Total Recall - Zed Testing Checklist

## Pre-Test Setup

- [ ] Binary installed and in PATH:
  ```bash
  cargo install --path crates/rag-mcp-server
  which rag-mcp
  ```

- [ ] Zed editor installed and running

- [ ] Extension installed in Zed:
  - Cmd+Shift+P → "zed: install dev extension"
  - Select: `/Users/vany/l/totalrecall/zed-extension`

## Verification Steps

- [ ] Open Zed Developer Console (Cmd+Option+I)
- [ ] Look for "Total Recall" or MCP server startup logs
- [ ] No error messages in console

## Test Cases

### Test 1: Store Memory
**Prompt to Claude in Zed:**
```
Remember this: Rust's ownership system prevents data races at compile time.
Tag it with: rust, memory-safety
```

**Expected:**
- [ ] Claude responds acknowledging the storage
- [ ] No errors in console
- [ ] Tool call visible in logs: `store_memory`

### Test 2: Search Memory
**Prompt:**
```
What do I know about Rust?
```

**Expected:**
- [ ] Claude finds and returns the memory about Rust
- [ ] Shows the content about ownership and data races
- [ ] Tool call visible: `search_memory`

### Test 3: Store Multiple Memories
**Prompt:**
```
Remember these facts:
1. BM25 is a probabilistic ranking function
2. Sled is an embedded database in Rust
Tag them with: search, databases
```

**Expected:**
- [ ] Claude stores both memories
- [ ] Confirms storage

### Test 4: List All Memories
**Prompt:**
```
List all my memories
```

**Expected:**
- [ ] Shows all 3 stored memories
- [ ] Includes tags for each
- [ ] Tool call visible: `list_memories`

### Test 5: Search with Multiple Results
**Prompt:**
```
Search for: rust database
```

**Expected:**
- [ ] Returns memories about both Rust and databases
- [ ] BM25 ranking visible (Sled memory scores higher)

### Test 6: Delete Memory
**Prompt:**
```
Delete the memory about BM25
```

**Expected:**
- [ ] Claude asks for confirmation or deletes
- [ ] Memory no longer appears in searches
- [ ] Tool call visible: `delete_memory`

### Test 7: Clear Session
**Prompt:**
```
Clear my session memories
```

**Expected:**
- [ ] Session cleared
- [ ] Project/global memories remain (if any)
- [ ] Tool call visible: `clear_session`

## Troubleshooting

If tests fail, check:

1. **Binary not found:**
   ```bash
   which rag-mcp
   # Should output path, if not:
   echo $PATH
   ```

2. **MCP server not starting:**
   - Check Zed console for errors
   - Test manually: `echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | rag-mcp serve`

3. **Tools not appearing:**
   - Restart Zed
   - Reinstall extension
   - Check extension is listed in Zed Extensions panel

4. **Errors in console:**
   - Copy error message
   - Check `rag-mcp serve` runs without errors
   - Verify JSON-RPC format

## Success Criteria

✅ All 7 tests pass
✅ No errors in Zed console
✅ Memories persist across prompts
✅ Search returns relevant results
✅ BM25 ranking works (most relevant first)

## After Testing

Once all tests pass:
- [ ] Document any issues found
- [ ] Note user experience observations
- [ ] Consider what features to add next
- [ ] Ready for official publication!

---

**Testing Date:** _____________

**Result:** ☐ Pass ☐ Fail ☐ Partial

**Notes:**

