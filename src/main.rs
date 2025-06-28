use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod db;
mod errors;
mod import;
mod metrics;
mod models;
mod search;
mod server;

use crate::config::Config;

#[derive(Parser)]
#[command(name = "llm-archive")]
#[command(about = "Fast, focused tool for searching LLM conversation archives", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Serve {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        
        #[arg(short, long, default_value = "./llm_archive.db")]
        database: PathBuf,
    },
    
    /// Import conversations from various formats
    Import {
        /// Provider type (chatgpt, claude, gemini, xai)
        provider: String,
        
        /// Path to export file(s)
        path: PathBuf,
        
        #[arg(short, long, default_value = "./llm_archive.db")]
        database: PathBuf,
        
        /// Use Python bridge for parsing (temporary)
        #[arg(long)]
        python_bridge: bool,
    },
    
    /// Search conversations
    Search {
        /// Search query
        query: String,
        
        #[arg(short, long, default_value = "./llm_archive.db")]
        database: PathBuf,
        
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    
    /// Initialize database
    Init {
        #[arg(short, long, default_value = "./llm_archive.db")]
        database: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .build();
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Serve { port, database } => {
            info!("Starting LLM Archive server on port {}", port);
            let config = Config::load()?;
            server::run(port, database, config).await?;
        }
        
        Commands::Import {
            provider,
            path,
            database,
            python_bridge,
        } => {
            info!("Importing {} conversations from {:?}", provider, path);
            let pool = db::create_pool(&database).await?;
            
            let start = std::time::Instant::now();
            let count = import::import_conversations(
                &pool,
                &provider,
                &path,
                python_bridge,
            ).await?;
            
            let elapsed = start.elapsed();
            info!(
                "Imported {} conversations in {:.2}s ({:.0} msgs/sec)",
                count,
                elapsed.as_secs_f64(),
                count as f64 / elapsed.as_secs_f64()
            );
        }
        
        Commands::Search { query, database, limit } => {
            let pool = db::create_pool(&database).await?;
            let results = search::search_conversations(&pool, &query, limit).await?;
            
            println!("Found {} results for '{}':", results.len(), query);
            for (i, conv) in results.iter().enumerate() {
                println!(
                    "{}. {} - {} ({})",
                    i + 1,
                    conv.title.as_deref().unwrap_or("Untitled"),
                    conv.provider,
                    conv.created_at.format("%Y-%m-%d")
                );
            }
        }
        
        Commands::Init { database } => {
            info!("Initializing database at {:?}", database);
            let pool = db::create_pool(&database).await?;
            db::run_migrations(&pool).await?;
            info!("Database initialized successfully");
        }
    }
    
    Ok(())
}