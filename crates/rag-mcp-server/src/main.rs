use anyhow::Result;
use clap::{Parser, Subcommand};
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
        #[arg(long, default_value = "session")]
        scope: String,
        #[arg(long)]
        tags: Vec<String>,
    },
    /// Search memories
    Search {
        query: String,
        #[arg(short, long, default_value = "5")]
        k: usize,
    },
    /// List memories
    List {
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// Delete memory
    Delete {
        id: String,
    },
    /// Show statistics
    Stats {
        #[arg(long)]
        scope: Option<String>,
    },
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rag_mcp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Serve => {
            info!("Starting MCP server...");
            error!("MCP server not implemented yet");
            anyhow::bail!("MCP server not implemented yet");
        }
        Commands::Add { content, scope, tags } => {
            info!("Adding memory: scope={}, tags={:?}", scope, tags);
            error!("Add command not implemented yet");
            anyhow::bail!("Add command not implemented yet");
        }
        Commands::Search { query, k } => {
            info!("Searching: query='{}', k={}", query, k);
            error!("Search command not implemented yet");
            anyhow::bail!("Search command not implemented yet");
        }
        Commands::List { scope, limit } => {
            info!("Listing memories: scope={:?}, limit={}", scope, limit);
            error!("List command not implemented yet");
            anyhow::bail!("List command not implemented yet");
        }
        Commands::Delete { id } => {
            info!("Deleting memory: id={}", id);
            error!("Delete command not implemented yet");
            anyhow::bail!("Delete command not implemented yet");
        }
        Commands::Stats { scope } => {
            info!("Getting stats: scope={:?}", scope);
            error!("Stats command not implemented yet");
            anyhow::bail!("Stats command not implemented yet");
        }
    }
}
