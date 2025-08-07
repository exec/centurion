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
mod history;
mod protocol;
mod security;
mod state;
mod utils;

use crate::actors::{ConnectionActor, ServerActor};
use crate::state::ServerState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ironchatd=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr = "127.0.0.1:6667".parse::<SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    
    info!("IRCv3 server listening on {}", addr);

    let server_state = Arc::new(RwLock::new(ServerState::new()));
    
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