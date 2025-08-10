use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod actors;
mod commands;
mod db;
mod error;
mod history;
mod legion;
mod protocol;
mod security;
mod state;
mod utils;

use crate::actors::{ConnectionActor, ServerActor};
use crate::state::ServerState;
use crate::utils::config::ServerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Print startup banner to stderr (not logged)
    eprintln!(r#"
    ╔═════════════════════════════════════════════╗
    ║                 CENTURION                   ║
    ║      Legion Protocol Enhanced IRC           ║
    ║                 v1.0.0                      ║
    ╚═════════════════════════════════════════════╝
    "#);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "centurion=info,legion_protocol=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Try to load config, otherwise use defaults
    let config_path = std::env::var("CENTURION_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());
    
    let config = if std::path::Path::new(&config_path).exists() {
        eprintln!("📁 Loading configuration from: {}", config_path);
        match ServerConfig::load(&config_path) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                eprintln!("⚠️  Failed to load config: {}. Using defaults.", e);
                None
            }
        }
    } else {
        eprintln!("ℹ️  No config file found. Using default settings.");
        None
    };

    // Use configured address or default
    let addr = if let Some(ref cfg) = config {
        if !cfg.server.listen_addresses.is_empty() {
            cfg.server.listen_addresses[0].parse::<SocketAddr>()?
        } else {
            "127.0.0.1:6667".parse::<SocketAddr>()?
        }
    } else {
        "127.0.0.1:6667".parse::<SocketAddr>()?
    };

    let listener = TcpListener::bind(addr).await?;
    
    // Print startup info to stderr 
    eprintln!("✅ Server started successfully!");
    eprintln!("📡 Listening on: {}", addr);
    eprintln!("🔐 TLS support: Available on port 6697");
    eprintln!("🚀 Legion Protocol: Enabled");
    eprintln!("");
    eprintln!("Press Ctrl+C to shutdown gracefully");
    eprintln!("{}", "-".repeat(60));
    
    info!("Centurion server with Legion Protocol starting on {}", addr);

    let mut server_state = ServerState::new();
    
    // Initialize Legion Protocol support
    if let Err(e) = server_state.init_legion().await {
        warn!("Legion Protocol initialization failed: {}. Running without Legion support.", e);
    }
    
    let server_state = Arc::new(RwLock::new(server_state));
    
    // Start server actor
    let (server_actor, _server_tx) = ServerActor::new(Arc::clone(&server_state));
    tokio::spawn(server_actor.run());

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!("New connection from {}", peer_addr);
                let state = Arc::clone(&server_state);
                
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, peer_addr, state).await {
                        error!("Connection error for {}: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                warn!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    server_state: Arc<RwLock<ServerState>>,
) -> Result<(), Box<dyn Error>> {
    let connection_id = {
        let state = server_state.read().await;
        state.generate_connection_id()
    };
    
    let connection_actor = ConnectionActor::new(
        connection_id,
        stream,
        peer_addr,
        server_state,
    ).await;
    
    connection_actor.run().await;
    
    Ok(())
}