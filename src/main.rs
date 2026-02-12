//! comm-node CLI entry point.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "comm-node", about = "FTL coordination for parallel AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold .orchestrator/ directories for all configured domains
    Init {
        /// Path to the project config file
        #[arg(long, default_value = "comm-node.toml")]
        config: PathBuf,
    },

    /// Start the orchestrator (watcher + router + lock manager)
    Start {
        /// Path to the project config file
        #[arg(long, default_value = "comm-node.toml")]
        config: PathBuf,
    },

    /// Show agent states, locks, and metrics
    Status,

    /// Graceful shutdown of a running orchestrator
    Stop,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { config } => {
            let project_config = comm_node::config::load(&config)?;
            tracing::info!(domains = project_config.domains.len(), "initializing comm-node");
            comm_node::scaffold::scaffold(&project_config)?;
            tracing::info!("scaffolding complete");
        }
        Command::Start { config } => {
            let project_config = comm_node::config::load(&config)?;
            let state_dir = dirs::home_dir()
                .expect("cannot determine home directory")
                .join(".comm-node/state");
            tracing::info!(
                domains = project_config.domains.len(),
                state_dir = %state_dir.display(),
                "starting comm-node"
            );
            let orchestrator =
                comm_node::orchestrator::Orchestrator::new(&project_config, state_dir)?;
            orchestrator.run().await?;
        }
        Command::Status => {
            println!("comm-node status: not yet implemented");
        }
        Command::Stop => {
            println!("comm-node stop: not yet implemented");
        }
    }

    Ok(())
}
