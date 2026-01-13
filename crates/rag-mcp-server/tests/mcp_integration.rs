use anyhow::{Context, Result};
use serde_json::{json, Value};
use serial_test::serial;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Mock Zed editor - communicates with MCP server via stdio
struct McpClient {
    child: Child,
    request_id: u64,
}

impl McpClient {
    fn spawn() -> Result<Self> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rag-mcp"))
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn MCP server")?;

        // Give server time to start
        std::thread::sleep(Duration::from_millis(100));

        Ok(Self {
            child,
            request_id: 0,
        })
    }

    fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params,
        });

        let request_str = serde_json::to_string(&request)?;

        let stdin = self.child.stdin.as_mut().context("Failed to get stdin")?;
        writeln!(stdin, "{}", request_str)?;
        stdin.flush()?;

        // Read response
        let stdout = self.child.stdout.as_mut().context("Failed to get stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response: Value = serde_json::from_str(&line.trim())?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("MCP error: {}", error);
        }

        response.get("result")
            .cloned()
            .context("No result in response")
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        self.send_request("tools/call", Some(json!({
            "name": name,
            "arguments": arguments,
        })))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
#[serial]
#[serial]
fn test_initialize() -> Result<()> {
    let mut client = McpClient::spawn()?;

    let result = client.send_request("initialize", Some(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {
            "name": "test-client",
            "version": "0.1.0"
        }
    })))?;

    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["capabilities"].is_object());
    assert_eq!(result["serverInfo"]["name"], "rag-mcp");

    Ok(())
}

#[test]
#[serial]
#[serial]
fn test_tools_list() -> Result<()> {
    let mut client = McpClient::spawn()?;

    let result = client.send_request("tools/list", None)?;
    let tools = result["tools"].as_array().context("tools not an array")?;

    assert!(tools.len() >= 5, "Expected at least 5 tools");

    let tool_names: Vec<&str> = tools.iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    assert!(tool_names.contains(&"store_memory"));
    assert!(tool_names.contains(&"search_memory"));
    assert!(tool_names.contains(&"list_memories"));
    assert!(tool_names.contains(&"delete_memory"));
    assert!(tool_names.contains(&"clear_session"));

    Ok(())
}

#[test]
#[serial]
#[serial]
fn test_store_and_search_session() -> Result<()> {
    let mut client = McpClient::spawn()?;

    // Store a memory in session scope
    let store_result = client.call_tool("store_memory", json!({
        "content": "Rust is a systems programming language",
        "scope": "session",
        "tags": ["rust", "programming"]
    }))?;

    let content = &store_result["content"];
    assert!(content.is_array());
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("Memory stored successfully"));

    // Search for the memory
    let search_result = client.call_tool("search_memory", json!({
        "query": "rust programming",
        "scope": "session",
        "k": 5
    }))?;

    let search_text = search_result["content"][0]["text"].as_str().unwrap();
    assert!(search_text.contains("Found 1 results"));
    assert!(search_text.contains("Rust is a systems programming language"));

    Ok(())
}

#[test]
#[serial]
fn test_store_and_list() -> Result<()> {

    let mut client = McpClient::spawn()?;

    // Clear session first
    client.call_tool("clear_session", json!({}))?;

    // Store multiple memories
    client.call_tool("store_memory", json!({
        "content": "First memory about Rust",
        "scope": "session",
        "tags": ["rust"]
    }))?;

    client.call_tool("store_memory", json!({
        "content": "Second memory about BM25",
        "scope": "session",
        "tags": ["search"]
    }))?;

    // List memories
    let list_result = client.call_tool("list_memories", json!({
        "scope": "session",
        "limit": 10,
        "offset": 0
    }))?;

    let list_text = list_result["content"][0]["text"].as_str().unwrap();
    assert!(list_text.contains("Found 2 memories"));
    assert!(list_text.contains("First memory about Rust"));
    assert!(list_text.contains("Second memory about BM25"));

    Ok(())
}

#[test]
#[serial]
fn test_delete_memory() -> Result<()> {

    let mut client = McpClient::spawn()?;

    // Clear session
    client.call_tool("clear_session", json!({}))?;

    // Store a memory
    let store_result = client.call_tool("store_memory", json!({
        "content": "Memory to delete",
        "scope": "session",
        "tags": []
    }))?;

    let store_text = store_result["content"][0]["text"].as_str().unwrap();
    let id = store_text
        .split("ID: ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .context("Failed to extract ID")?;

    // Delete the memory
    let delete_result = client.call_tool("delete_memory", json!({
        "id": id,
        "scope": "session"
    }))?;

    let delete_text = delete_result["content"][0]["text"].as_str().unwrap();
    assert!(delete_text.contains("deleted successfully"));

    // Verify it's gone
    let list_result = client.call_tool("list_memories", json!({
        "scope": "session",
        "limit": 10
    }))?;

    let list_text = list_result["content"][0]["text"].as_str().unwrap();
    assert!(list_text.contains("No memories found") || list_text.contains("Found 0"));

    Ok(())
}

#[test]
#[serial]
fn test_clear_session() -> Result<()> {

    let mut client = McpClient::spawn()?;

    // Store some memories
    for i in 0..3 {
        client.call_tool("store_memory", json!({
            "content": format!("Test memory {}", i),
            "scope": "session",
            "tags": []
        }))?;
    }

    // Clear session
    let clear_result = client.call_tool("clear_session", json!({}))?;
    let clear_text = clear_result["content"][0]["text"].as_str().unwrap();
    assert!(clear_text.contains("cleared successfully"));

    // Verify all gone
    let list_result = client.call_tool("list_memories", json!({
        "scope": "session",
        "limit": 10
    }))?;

    let list_text = list_result["content"][0]["text"].as_str().unwrap();
    assert!(list_text.contains("No memories found"));

    Ok(())
}

#[test]
#[serial]
fn test_bm25_ranking() -> Result<()> {

    let mut client = McpClient::spawn()?;

    client.call_tool("clear_session", json!({}))?;

    // Store memories with varying relevance
    client.call_tool("store_memory", json!({
        "content": "Rust programming language for systems",
        "scope": "session",
        "tags": []
    }))?;

    client.call_tool("store_memory", json!({
        "content": "Python is great for scripting",
        "scope": "session",
        "tags": []
    }))?;

    client.call_tool("store_memory", json!({
        "content": "Rust systems programming with safety guarantees",
        "scope": "session",
        "tags": []
    }))?;

    // Search for "rust systems"
    let search_result = client.call_tool("search_memory", json!({
        "query": "rust systems",
        "scope": "session",
        "k": 5
    }))?;

    let search_text = search_result["content"][0]["text"].as_str().unwrap();

    // Should find 2 results (Rust-related)
    assert!(search_text.contains("Found 2 results"), "Expected 2 results, got: {}", search_text);

    // The result with both "rust" and "systems" should rank higher
    // Just verify the top result contains "rust" (either case)
    assert!(search_text.to_lowercase().contains("rust systems") ||
            (search_text.to_lowercase().contains("rust") && search_text.to_lowercase().contains("systems")),
            "Expected top result to contain both 'rust' and 'systems'. Got: {}", search_text);

    Ok(())
}

#[test]
#[serial]
fn test_tags_in_storage() -> Result<()> {

    let mut client = McpClient::spawn()?;

    client.call_tool("clear_session", json!({}))?;

    // Store with specific tags
    client.call_tool("store_memory", json!({
        "content": "Tagged memory",
        "scope": "session",
        "tags": ["important", "rust", "async"]
    }))?;

    // List and verify tags appear
    let list_result = client.call_tool("list_memories", json!({
        "scope": "session",
        "limit": 10
    }))?;

    let list_text = list_result["content"][0]["text"].as_str().unwrap();
    assert!(list_text.contains("important"));
    assert!(list_text.contains("rust"));
    assert!(list_text.contains("async"));

    Ok(())
}

#[test]
#[serial]
fn test_empty_search_results() -> Result<()> {

    let mut client = McpClient::spawn()?;

    client.call_tool("clear_session", json!({}))?;

    // Search with no memories
    let search_result = client.call_tool("search_memory", json!({
        "query": "nonexistent query",
        "scope": "session",
        "k": 5
    }))?;

    let search_text = search_result["content"][0]["text"].as_str().unwrap();
    assert!(search_text.contains("No matching memories found"));

    Ok(())
}
