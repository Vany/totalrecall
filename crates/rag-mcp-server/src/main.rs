mod mcp;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rag_core::{config::Config, storage::MemoryStore, Memory, MemoryMetadata, MemoryScope};
use rag_search::BM25SearchEngine;
use server::McpServer;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "rag-mcp")]
#[command(about = "RAG MCP Server for Zed/Claude Code", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run MCP server (stdio)
    Serve,
    /// Add memory
    Add {
        #[arg(long)]
        content: String,
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        tags: Vec<String>,
        #[arg(long)]
        project_path: Option<PathBuf>,
    },
    /// Search memories
    Search {
        query: String,
        #[arg(short, long, default_value = "5")]
        k: usize,
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        project_path: Option<PathBuf>,
    },
    /// List memories
    List {
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long, default_value = "50")]
        limit: usize,
        #[arg(long)]
        project_path: Option<PathBuf>,
    },
    /// Delete memory
    Delete {
        id: String,
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        project_path: Option<PathBuf>,
    },
    /// Show statistics
    Stats {
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        project_path: Option<PathBuf>,
    },
}

fn init_tracing(stderr_only: bool) {
    if stderr_only {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "rag_mcp=warn".into()),
            )
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "rag_mcp=info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

fn parse_scope(scope: &str, project_path: Option<PathBuf>) -> Result<MemoryScope> {
    match scope {
        "session" => Ok(MemoryScope::Session),
        "global" => Ok(MemoryScope::Global),
        "project" => {
            let path = project_path.ok_or_else(|| anyhow::anyhow!("project_path required for project scope"))?;
            Ok(MemoryScope::Project { path })
        }
        _ => anyhow::bail!("Invalid scope: {}. Use session, project, or global", scope),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // For serve mode, send logs to stderr to keep stdout clean for JSON-RPC
    let stderr_only = matches!(cli.command, Commands::Serve);
    init_tracing(stderr_only);

    match cli.command {
        Commands::Serve => {
            let config = Config::load()?;
            let mut server = McpServer::new(config)?;
            server.run()?;
        }
        Commands::Add { content, scope, tags, project_path } => {
            let config = Config::load()?;
            let mut store = MemoryStore::new(config.storage.global_db_path)?;
            let scope = parse_scope(&scope, project_path)?;

            let metadata = MemoryMetadata {
                tags,
                ..Default::default()
            };

            let memory = Memory::new(content, scope, metadata);
            let id = memory.id.clone();

            store.store(memory)?;
            info!("Memory stored with ID: {}", id);
        }
        Commands::Search { query, k, scope, project_path } => {
            let config = Config::load()?;
            let store = MemoryStore::new(config.storage.global_db_path)?;
            let scope = parse_scope(&scope, project_path)?;

            let memories = store.list_all(&scope)?;
            let mut search = BM25SearchEngine::new();

            for memory in &memories {
                search.index_memory(memory);
            }

            let results = search.search(&query, &memories, k);

            if results.is_empty() {
                info!("No results found");
            } else {
                info!("Found {} results:", results.len());
                for result in results {
                    println!("\nScore: {:.2}", result.score);
                    println!("ID: {}", result.memory.id);
                    println!("Content: {}", result.memory.content);
                    println!("---");
                }
            }
        }
        Commands::List { scope, limit, project_path } => {
            let config = Config::load()?;
            let store = MemoryStore::new(config.storage.global_db_path)?;
            let scope = parse_scope(&scope, project_path)?;

            let memories = store.list(&scope, limit, 0)?;

            if memories.is_empty() {
                info!("No memories found");
            } else {
                info!("Found {} memories:", memories.len());
                for memory in memories {
                    println!("\nID: {}", memory.id);
                    println!("Tags: {}", memory.metadata.tags.join(", "));
                    println!("Content: {}", memory.content);
                    println!("---");
                }
            }
        }
        Commands::Delete { id, scope, project_path } => {
            let config = Config::load()?;
            let mut store = MemoryStore::new(config.storage.global_db_path)?;
            let scope = parse_scope(&scope, project_path)?;

            let deleted = store.delete(&id, &scope)?;
            if deleted {
                info!("Memory {} deleted", id);
            } else {
                error!("Memory {} not found", id);
            }
        }
        Commands::Stats { scope, project_path } => {
            let config = Config::load()?;
            let store = MemoryStore::new(config.storage.global_db_path)?;
            let scope = parse_scope(&scope, project_path)?;

            let stats = store.stats(&scope)?;
            info!("Total memories: {}", stats.total_memories);
        }
    }

    Ok(())
}
