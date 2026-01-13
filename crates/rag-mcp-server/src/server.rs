use anyhow::{Context, Result};
use rag_core::{config::Config, storage::MemoryStore, Memory, MemoryMetadata, MemoryScope};
use rag_search::BM25SearchEngine;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use tracing::{debug, error, info};

use crate::mcp::{JsonRpcRequest, JsonRpcResponse, Tool};

pub struct McpServer {
    config: Config,
    store: MemoryStore,
    search: BM25SearchEngine,
}

impl McpServer {
    pub fn new(config: Config) -> Result<Self> {
        let store = MemoryStore::new(config.storage.global_db_path.clone())?;
        let search = BM25SearchEngine::new();

        Ok(Self {
            config,
            store,
            search,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting MCP server on stdio");

        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut stdout = std::io::stdout();

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    info!("EOF received, shutting down");
                    break;
                }
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    debug!("Received: {}", line);

                    match serde_json::from_str::<JsonRpcRequest>(line) {
                        Ok(request) => {
                            // Handle notifications (no response needed)
                            if request.id.is_none() {
                                debug!("Received notification: {}", request.method);
                                if request.method.starts_with("notifications/") {
                                    // Silently ignore notifications
                                    continue;
                                }
                            }

                            // Handle requests (response needed)
                            let response = self.handle_request(request);
                            let response_str = serde_json::to_string(&response)?;
                            writeln!(stdout, "{}", response_str)?;
                            stdout.flush()?;
                        }
                        Err(e) => {
                            error!("Failed to parse request: {}", e);
                            let response =
                                JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                            let response_str = serde_json::to_string(&response)?;
                            writeln!(stdout, "{}", response_str)?;
                            stdout.flush()?;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read line: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling method: {}", request.method);

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(request.params),
            "resources/list" => self.handle_resources_list(),
            "resources/read" => self.handle_resources_read(request.params),
            _ => Err(anyhow::anyhow!("Method not found: {}", request.method)),
        };

        match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(e) => {
                error!("Error handling request: {}", e);
                JsonRpcResponse::error(request.id, -32603, format!("Internal error: {}", e))
            }
        }
    }

    fn handle_initialize(&self, _params: Option<Value>) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "rag-mcp",
                "version": "0.1.0"
            }
        }))
    }

    fn handle_tools_list(&self) -> Result<Value> {
        let tools = vec![
            Tool {
                name: "store_memory".to_string(),
                description: "Store new memory with metadata".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": {"type": "string", "description": "Content to store"},
                        "scope": {
                            "type": "string",
                            "enum": ["session", "project", "global"],
                            "description": "Memory scope"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Tags for categorization"
                        },
                        "project_path": {
                            "type": "string",
                            "description": "Project path (required for project scope)"
                        }
                    },
                    "required": ["content", "scope"]
                }),
            },
            Tool {
                name: "search_memory".to_string(),
                description: "Search memories using BM25 keyword search".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "scope": {
                            "type": "string",
                            "enum": ["session", "project", "global"],
                            "description": "Memory scope to search"
                        },
                        "k": {
                            "type": "integer",
                            "description": "Number of results to return",
                            "default": 5
                        },
                        "project_path": {
                            "type": "string",
                            "description": "Project path (required for project scope)"
                        }
                    },
                    "required": ["query", "scope"]
                }),
            },
            Tool {
                name: "list_memories".to_string(),
                description: "List memories with pagination".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "scope": {"type": "string", "enum": ["session", "project", "global"]},
                        "limit": {"type": "integer", "default": 50},
                        "offset": {"type": "integer", "default": 0},
                        "project_path": {"type": "string"}
                    },
                    "required": ["scope"]
                }),
            },
            Tool {
                name: "delete_memory".to_string(),
                description: "Delete memory by ID".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "scope": {"type": "string", "enum": ["session", "project", "global"]},
                        "project_path": {"type": "string"}
                    },
                    "required": ["id", "scope"]
                }),
            },
            Tool {
                name: "clear_session".to_string(),
                description: "Clear all session memories".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

        Ok(json!({ "tools": tools }))
    }

    fn handle_tools_call(&mut self, params: Option<Value>) -> Result<Value> {
        let params = params.context("Missing params")?;
        let name = params["name"].as_str().context("Missing tool name")?;
        let arguments = &params["arguments"];

        match name {
            "store_memory" => self.tool_store_memory(arguments),
            "search_memory" => self.tool_search_memory(arguments),
            "list_memories" => self.tool_list_memories(arguments),
            "delete_memory" => self.tool_delete_memory(arguments),
            "clear_session" => self.tool_clear_session(),
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    fn tool_store_memory(&mut self, args: &Value) -> Result<Value> {
        let content = args["content"].as_str().context("Missing content")?;
        let scope_str = args["scope"].as_str().context("Missing scope")?;
        let tags: Vec<String> = args["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let scope = match scope_str {
            "session" => MemoryScope::Session,
            "global" => MemoryScope::Global,
            "project" => {
                let path = args["project_path"]
                    .as_str()
                    .context("Missing project_path for project scope")?;
                MemoryScope::Project {
                    path: PathBuf::from(path),
                }
            }
            _ => return Err(anyhow::anyhow!("Invalid scope: {}", scope_str)),
        };

        let metadata = MemoryMetadata {
            tags,
            ..Default::default()
        };

        let memory = Memory::new(content.to_string(), scope, metadata);
        let id = memory.id.clone();

        self.search.index_memory(&memory);
        self.store.store(memory)?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("Memory stored successfully with ID: {}", id)
            }]
        }))
    }

    fn tool_search_memory(&mut self, args: &Value) -> Result<Value> {
        let query = args["query"].as_str().context("Missing query")?;
        let scope_str = args["scope"].as_str().context("Missing scope")?;
        let k = args["k"]
            .as_u64()
            .unwrap_or(self.config.search.default_k as u64) as usize;

        let scope = match scope_str {
            "session" => MemoryScope::Session,
            "global" => MemoryScope::Global,
            "project" => {
                let path = args["project_path"]
                    .as_str()
                    .context("Missing project_path for project scope")?;
                MemoryScope::Project {
                    path: PathBuf::from(path),
                }
            }
            _ => return Err(anyhow::anyhow!("Invalid scope: {}", scope_str)),
        };

        let all_memories = self.store.list_all(&scope)?;
        let results = self.search.search(query, &all_memories, k);

        let results_text = if results.is_empty() {
            "No matching memories found.".to_string()
        } else {
            let mut output = format!("Found {} results:\n\n", results.len());
            for result in &results {
                output.push_str(&format!(
                    "Score: {:.2} | ID: {}\n{}\n\n---\n\n",
                    result.score, result.memory.id, result.memory.content
                ));
            }
            output
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": results_text
            }]
        }))
    }

    fn tool_list_memories(&self, args: &Value) -> Result<Value> {
        let scope_str = args["scope"].as_str().context("Missing scope")?;
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;
        let offset = args["offset"].as_u64().unwrap_or(0) as usize;

        let scope = match scope_str {
            "session" => MemoryScope::Session,
            "global" => MemoryScope::Global,
            "project" => {
                let path = args["project_path"]
                    .as_str()
                    .context("Missing project_path for project scope")?;
                MemoryScope::Project {
                    path: PathBuf::from(path),
                }
            }
            _ => return Err(anyhow::anyhow!("Invalid scope: {}", scope_str)),
        };

        let memories = self.store.list(&scope, limit, offset)?;

        let text = if memories.is_empty() {
            "No memories found.".to_string()
        } else {
            let mut output = format!("Found {} memories:\n\n", memories.len());
            for memory in &memories {
                output.push_str(&format!(
                    "ID: {} | Tags: {}\n{}\n\n---\n\n",
                    memory.id,
                    memory.metadata.tags.join(", "),
                    memory.content
                ));
            }
            output
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": text
            }]
        }))
    }

    fn tool_delete_memory(&mut self, args: &Value) -> Result<Value> {
        let id = args["id"].as_str().context("Missing id")?;
        let scope_str = args["scope"].as_str().context("Missing scope")?;

        let scope = match scope_str {
            "session" => MemoryScope::Session,
            "global" => MemoryScope::Global,
            "project" => {
                let path = args["project_path"]
                    .as_str()
                    .context("Missing project_path for project scope")?;
                MemoryScope::Project {
                    path: PathBuf::from(path),
                }
            }
            _ => return Err(anyhow::anyhow!("Invalid scope: {}", scope_str)),
        };

        let deleted = self.store.delete(id, &scope)?;
        if deleted {
            self.search.remove_memory(id);
        }

        let text = if deleted {
            format!("Memory {} deleted successfully", id)
        } else {
            format!("Memory {} not found", id)
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": text
            }]
        }))
    }

    fn tool_clear_session(&mut self) -> Result<Value> {
        self.store.clear_session();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Session memories cleared successfully"
            }]
        }))
    }

    fn handle_resources_list(&self) -> Result<Value> {
        Ok(json!({ "resources": [] }))
    }

    fn handle_resources_read(&self, _params: Option<Value>) -> Result<Value> {
        Err(anyhow::anyhow!("No resources available"))
    }
}
