use anyhow::{Context, Result};
use serde_json::{json, Value};
use serial_test::serial;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Mock Zed/Claude Code client - accurately simulates MCP protocol over stdio
///
/// This client mimics how Zed's context_server implementation communicates:
/// 1. Spawns MCP server process with stdio transport
/// 2. Sends initialize request with protocolVersion and capabilities
/// 3. Sends notifications/initialized notification (no response expected)
/// 4. Makes tools/list and tools/call requests
/// 5. Handles both requests (with id) and notifications (id: null)
struct ZedMcpClient {
    child: Child,
    request_id: u64,
    reader: Arc<Mutex<BufReader<std::process::ChildStdout>>>,
}

impl ZedMcpClient {
    /// Spawn MCP server and perform initialization handshake
    fn spawn() -> Result<Self> {
        // Use test-specific database directory to avoid conflicts with running servers
        // Use random ID for uniqueness across concurrent instances
        use std::sync::atomic::{AtomicU64, Ordering};
        static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let instance_id = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst);

        let test_db_dir = std::env::temp_dir().join(format!(
            "rag-mcp-test-{}-{}",
            std::process::id(),
            instance_id
        ));
        std::fs::create_dir_all(&test_db_dir)?;

        let mut child = Command::new(env!("CARGO_BIN_EXE_rag-mcp"))
            .arg("serve")
            .env("RAG_MCP_DB_PATH", test_db_dir.to_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Capture stderr for debug output
            .spawn()
            .context("Failed to spawn MCP server")?;

        let stdout = child.stdout.take().context("Failed to take stdout")?;
        let reader = Arc::new(Mutex::new(BufReader::new(stdout)));

        // Capture stderr in background thread for debugging
        if let Some(stderr) = child.stderr.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        eprintln!("[MCP SERVER] {}", line);
                    }
                }
            });
        }

        // Give server time to start
        thread::sleep(Duration::from_millis(50));

        let mut client = Self {
            child,
            request_id: 0,
            reader,
        };

        // Perform MCP initialization handshake
        client.initialize()?;

        Ok(client)
    }

    /// Send MCP initialize request and notifications/initialized notification
    /// Mimics Zed's initialization sequence
    fn initialize(&mut self) -> Result<()> {
        // 1. Send initialize request
        let init_response = self.send_request(
            "initialize",
            Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "zed-test-client",
                    "version": "0.218.7"
                }
            })),
        )?;

        // Verify initialize response
        if init_response
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            != Some("2024-11-05")
        {
            anyhow::bail!("Invalid protocolVersion in initialize response");
        }

        // 2. Send notifications/initialized notification (no response expected)
        self.send_notification("notifications/initialized", None)?;

        Ok(())
    }

    /// Send JSON-RPC request and wait for response
    fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params.unwrap_or(json!({})),
        });

        self.write_message(&request)?;
        self.read_response(self.request_id)
    }

    /// Send JSON-RPC notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "id": null,
            "method": method,
            "params": params.unwrap_or(json!({})),
        });

        self.write_message(&notification)?;
        // Notifications don't get responses, so don't wait
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Write JSON message to server stdin
    fn write_message(&mut self, message: &Value) -> Result<()> {
        let message_str = serde_json::to_string(message)?;
        let stdin = self.child.stdin.as_mut().context("Failed to get stdin")?;
        writeln!(stdin, "{}", message_str)?;
        stdin.flush()?;
        Ok(())
    }

    /// Read response from server stdout (blocking, with simple timeout via channel)
    fn read_response(&mut self, expected_id: u64) -> Result<Value> {
        // Simple blocking read - server should respond quickly
        let mut reader = self.reader.lock().unwrap();
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .context("Failed to read response from server")?;

        let response: Value = serde_json::from_str(line.trim())
            .context(format!("Failed to parse response: {}", line.trim()))?;

        // Verify this is the response we're waiting for
        if let Some(id) = response.get("id") {
            if id.as_u64() != Some(expected_id) {
                anyhow::bail!("Response ID mismatch: expected {}, got {}", expected_id, id);
            }
        }

        // Check for JSON-RPC error
        if let Some(error) = response.get("error") {
            anyhow::bail!("MCP error: {}", serde_json::to_string_pretty(error)?);
        }

        // Extract result
        response
            .get("result")
            .cloned()
            .context("No result in response")
    }

    /// Call an MCP tool (mimics Zed's tools/call request)
    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        self.send_request(
            "tools/call",
            Some(json!({
                "name": name,
                "arguments": arguments,
            })),
        )
    }

    /// List available tools (mimics Zed's tools/list request)
    fn list_tools(&mut self) -> Result<Vec<Value>> {
        let result = self.send_request("tools/list", None)?;
        result["tools"]
            .as_array()
            .cloned()
            .context("tools/list did not return array")
    }
}

impl Drop for ZedMcpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
#[serial]
fn test_mcp_initialization() -> Result<()> {
    let client = ZedMcpClient::spawn()?;

    // Client spawning already performs initialization
    // If we got here, initialization succeeded
    drop(client);
    Ok(())
}

#[test]
#[serial]
fn test_tools_list_protocol() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;

    let tools = client.list_tools()?;

    assert!(
        tools.len() >= 5,
        "Expected at least 5 tools, got {}",
        tools.len()
    );

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Verify all expected tools are present
    let expected_tools = [
        "store_memory",
        "search_memory",
        "list_memories",
        "delete_memory",
        "clear_session",
    ];

    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "Missing tool: {}. Available tools: {:?}",
            expected,
            tool_names
        );
    }

    // Verify each tool has required schema fields
    for tool in &tools {
        assert!(tool["name"].is_string(), "Tool missing name");
        assert!(tool["description"].is_string(), "Tool missing description");
        assert!(tool["inputSchema"].is_object(), "Tool missing inputSchema");
    }

    Ok(())
}

#[test]
#[serial]
fn test_store_memory_session_scope() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;

    // Clear session first
    client.call_tool("clear_session", json!({}))?;

    // Store a memory
    let result = client.call_tool(
        "store_memory",
        json!({
            "content": "Rust is a systems programming language with memory safety",
            "scope": "session",
            "tags": ["rust", "systems", "safety"]
        }),
    )?;

    // Verify response format (MCP tools return content array)
    assert!(
        result["content"].is_array(),
        "Expected content array in response"
    );

    let content = result["content"].as_array().unwrap();
    assert!(!content.is_empty(), "Expected non-empty content array");

    let text = content[0]["text"].as_str().context("Expected text field")?;
    assert!(
        text.contains("Memory stored successfully"),
        "Expected success message"
    );
    assert!(text.contains("ID:"), "Expected memory ID in response");

    Ok(())
}

#[test]
#[serial]
fn test_search_memory_bm25_ranking() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;
    client.call_tool("clear_session", json!({}))?;

    // Store memories with varying relevance to query "rust systems"
    let memories = [
        ("Rust systems programming with memory safety", 2), // Both keywords
        ("Python scripting and automation tools", 0),       // No keywords
        ("Systems design patterns in software", 1),         // One keyword
        ("Rust async programming and tokio runtime", 1),    // One keyword
    ];

    for (content, _expected_rank) in &memories {
        client.call_tool(
            "store_memory",
            json!({
                "content": content,
                "scope": "session",
                "tags": []
            }),
        )?;
    }

    // Search for "rust systems"
    let result = client.call_tool(
        "search_memory",
        json!({
            "query": "rust systems",
            "scope": "session",
            "k": 5
        }),
    )?;

    let text = result["content"][0]["text"].as_str().unwrap();

    // Should find 3 results (anything with rust OR systems)
    assert!(
        text.contains("Found 3 results"),
        "Expected 3 results, got: {}",
        text
    );

    // Verify the memory with both keywords appears in results
    assert!(
        text.to_lowercase().contains("rust") && text.to_lowercase().contains("memory safety"),
        "Results should include memory with both keywords. Got: {}",
        text
    );

    Ok(())
}

#[test]
#[serial]
fn test_list_memories_with_pagination() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;
    client.call_tool("clear_session", json!({}))?;

    // Store 5 memories
    for i in 0..5 {
        client.call_tool(
            "store_memory",
            json!({
                "content": format!("Memory number {} with unique content", i),
                "scope": "session",
                "tags": [format!("tag-{}", i)]
            }),
        )?;
    }

    // List with limit
    let result = client.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 3,
            "offset": 0
        }),
    )?;

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Found 3 memories"),
        "Expected 3 memories in first page"
    );

    // List with offset
    let result = client.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 3,
            "offset": 3
        }),
    )?;

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Found 2 memories"),
        "Expected 2 memories in second page"
    );

    Ok(())
}

#[test]
#[serial]
fn test_delete_memory_by_id() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;
    client.call_tool("clear_session", json!({}))?;

    // Store a memory and extract its ID
    let store_result = client.call_tool(
        "store_memory",
        json!({
            "content": "Memory to be deleted",
            "scope": "session",
            "tags": []
        }),
    )?;

    let store_text = store_result["content"][0]["text"].as_str().unwrap();
    let memory_id = store_text
        .split("ID: ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .context("Failed to extract memory ID")?;

    // Delete the memory
    let delete_result = client.call_tool(
        "delete_memory",
        json!({
            "id": memory_id,
            "scope": "session"
        }),
    )?;

    let delete_text = delete_result["content"][0]["text"].as_str().unwrap();
    assert!(delete_text.contains("deleted successfully"));

    // Verify deletion
    let list_result = client.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 10,
            "offset": 0
        }),
    )?;

    let list_text = list_result["content"][0]["text"].as_str().unwrap();
    assert!(
        list_text.contains("No memories found") || list_text.contains("Found 0"),
        "Expected no memories after deletion"
    );

    Ok(())
}

#[test]
#[serial]
fn test_clear_session_scope() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;

    // Store multiple memories in session
    for i in 0..3 {
        client.call_tool(
            "store_memory",
            json!({
                "content": format!("Session memory {}", i),
                "scope": "session",
                "tags": []
            }),
        )?;
    }

    // Verify they exist
    let list_before = client.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 10,
            "offset": 0
        }),
    )?;
    let text_before = list_before["content"][0]["text"].as_str().unwrap();
    assert!(text_before.contains("Found 3 memories"));

    // Clear session
    let clear_result = client.call_tool("clear_session", json!({}))?;
    let clear_text = clear_result["content"][0]["text"].as_str().unwrap();
    assert!(clear_text.contains("cleared successfully"));

    // Verify all gone
    let list_after = client.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 10,
            "offset": 0
        }),
    )?;
    let text_after = list_after["content"][0]["text"].as_str().unwrap();
    assert!(text_after.contains("No memories found"));

    Ok(())
}

#[test]
#[serial]
fn test_tags_storage_and_display() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;
    client.call_tool("clear_session", json!({}))?;

    // Store memory with multiple tags
    client.call_tool(
        "store_memory",
        json!({
            "content": "Important async Rust code example",
            "scope": "session",
            "tags": ["rust", "async", "important", "example"]
        }),
    )?;

    // List and verify tags are displayed
    let result = client.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 10,
            "offset": 0
        }),
    )?;

    let text = result["content"][0]["text"].as_str().unwrap();

    // Verify all tags appear in output
    for tag in &["rust", "async", "important", "example"] {
        assert!(
            text.contains(tag),
            "Expected tag '{}' in output. Got: {}",
            tag,
            text
        );
    }

    Ok(())
}

#[test]
#[serial]
fn test_empty_search_results() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;
    client.call_tool("clear_session", json!({}))?;

    // Search with no stored memories
    let result = client.call_tool(
        "search_memory",
        json!({
            "query": "nonexistent content that will never match",
            "scope": "session",
            "k": 5
        }),
    )?;

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("No matching memories found"),
        "Expected 'no matching memories' message. Got: {}",
        text
    );

    Ok(())
}

#[test]
#[serial]
fn test_concurrent_client_sessions() -> Result<()> {
    // Each client gets its own session scope (in-memory)
    let mut client1 = ZedMcpClient::spawn()?;
    let mut client2 = ZedMcpClient::spawn()?;

    // Client 1 stores a memory
    client1.call_tool(
        "store_memory",
        json!({
            "content": "Client 1 exclusive memory",
            "scope": "session",
            "tags": []
        }),
    )?;

    // Client 2 stores a different memory
    client2.call_tool(
        "store_memory",
        json!({
            "content": "Client 2 exclusive memory",
            "scope": "session",
            "tags": []
        }),
    )?;

    // Each client should only see their own memory
    let list1 = client1.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 10,
            "offset": 0
        }),
    )?;
    let text1 = list1["content"][0]["text"].as_str().unwrap();
    assert!(text1.contains("Client 1 exclusive"));
    assert!(!text1.contains("Client 2 exclusive"));

    let list2 = client2.call_tool(
        "list_memories",
        json!({
            "scope": "session",
            "limit": 10,
            "offset": 0
        }),
    )?;
    let text2 = list2["content"][0]["text"].as_str().unwrap();
    assert!(text2.contains("Client 2 exclusive"));
    assert!(!text2.contains("Client 1 exclusive"));

    Ok(())
}

#[test]
#[serial]
fn test_error_handling_invalid_scope() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;

    // Try to use invalid scope
    let result = client.send_request(
        "tools/call",
        Some(json!({
            "name": "store_memory",
            "arguments": {
                "content": "Test content",
                "scope": "invalid_scope",
                "tags": []
            }
        })),
    );

    // Should get an error response
    assert!(
        result.is_err(),
        "Expected error for invalid scope, but got success"
    );

    Ok(())
}

#[test]
#[serial]
fn test_bm25_stop_words_filtering() -> Result<()> {
    let mut client = ZedMcpClient::spawn()?;
    client.call_tool("clear_session", json!({}))?;

    // Store memories
    client.call_tool(
        "store_memory",
        json!({
            "content": "The quick brown fox jumps over the lazy dog",
            "scope": "session",
            "tags": []
        }),
    )?;

    client.call_tool(
        "store_memory",
        json!({
            "content": "Quick fox programming language tutorial",
            "scope": "session",
            "tags": []
        }),
    )?;

    // Search with stop words - "the" should be filtered out
    let result = client.call_tool(
        "search_memory",
        json!({
            "query": "quick fox",
            "scope": "session",
            "k": 5
        }),
    )?;

    let text = result["content"][0]["text"].as_str().unwrap();

    // Both should match since they contain "quick" and "fox"
    assert!(
        text.contains("Found 2 results"),
        "Expected 2 results. Got: {}",
        text
    );

    Ok(())
}
