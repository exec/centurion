use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::BytesMut;
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

use crate::protocol::{Command, IrcCodec, Message, Reply};
use crate::security::RateLimiter;
use crate::state::{Connection, ServerState};

const PING_INTERVAL: Duration = Duration::from_secs(120);
const PING_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_MESSAGE_RATE: u32 = 10; // messages per second

pub struct ConnectionActor {
    id: u64,
    addr: SocketAddr,
    stream: Framed<TcpStream, IrcCodec>,
    server_state: Arc<RwLock<ServerState>>,
    rx: mpsc::Receiver<Message>,
    tx: mpsc::Sender<Message>,
    rate_limiter: RateLimiter,
    ping_token: Option<String>,
    registered: bool,
    capabilities_negotiating: bool,
    capabilities_enabled: Vec<String>,
}

impl ConnectionActor {
    pub async fn new(
        id: u64,
        tcp_stream: TcpStream,
        addr: SocketAddr,
        server_state: Arc<RwLock<ServerState>>,
    ) -> Self {
        let stream = Framed::new(tcp_stream, IrcCodec::new());
        let (tx, rx) = mpsc::channel(256);
        
        // Register connection in server state
        {
            let mut state = server_state.write().await;
            state.connections.insert(id, Connection::new(id, addr, tx.clone()));
        }
        
        Self {
            id,
            addr,
            stream,
            server_state,
            rx,
            tx,
            rate_limiter: RateLimiter::new(MAX_MESSAGE_RATE, Duration::from_secs(1)),
            ping_token: None,
            registered: false,
            capabilities_negotiating: false,
            capabilities_enabled: Vec::new(),
        }
    }
    
    pub async fn run(mut self) {
        info!("Connection actor {} started for {}", self.id, self.addr);
        
        let mut ping_interval = interval(PING_INTERVAL);
        ping_interval.tick().await; // Skip first immediate tick
        
        loop {
            tokio::select! {
                // Handle incoming messages from client
                result = self.stream.next() => {
                    match result {
                        Some(Ok(msg)) => {
                            if !self.rate_limiter.check().await {
                                self.send_error("Flood protection triggered").await;
                                break;
                            }
                            
                            if let Err(e) = self.handle_client_message(msg).await {
                                error!("Error handling message: {}", e);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("Error reading from stream: {}", e);
                            break;
                        }
                        None => {
                            info!("Client {} disconnected", self.addr);
                            break;
                        }
                    }
                }
                
                // Handle messages to send to client
                Some(msg) = self.rx.recv() => {
                    if let Err(e) = self.stream.send(msg).await {
                        error!("Error sending message: {}", e);
                        break;
                    }
                }
                
                // Send periodic PING
                _ = ping_interval.tick() => {
                    if let Err(e) = self.send_ping().await {
                        error!("Error sending ping: {}", e);
                        break;
                    }
                }
            }
        }
        
        self.cleanup().await;
        info!("Connection actor {} stopped", self.id);
    }
    
    async fn handle_client_message(&mut self, msg: Message) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Received from {}: {:?}", self.addr, msg);
        
        // Update activity timestamp
        {
            let mut state = self.server_state.write().await;
            if let Some(mut conn) = state.connections.get_mut(&self.id) {
                conn.update_activity();
                drop(conn);
            }
            drop(state);
        }
        
        let command = Command::parse(&msg.command, msg.params);
        
        match command {
            Command::Cap { subcommand, params } => {
                self.handle_cap(subcommand, params).await?;
            }
            Command::Nick(nick) => {
                // Handle NICK locally to avoid interfering with CAP negotiation
                self.handle_nick(nick).await?;
            }
            Command::User { username, realname } => {
                self.handle_user(username, realname).await?;
            }
            Command::Ping(token) => {
                self.handle_ping(token).await?;
            }
            Command::Pong(token) => {
                self.handle_pong(token).await?;
            }
            Command::Quit(reason) => {
                self.handle_quit(reason).await?;
                return Err("Client quit".into());
            }
            _ => {
                if !self.registered {
                    self.send_reply(Reply::NotRegistered { 
                        nick: "*".to_string() 
                    }).await?;
                } else {
                    // Handle other commands through command processor
                    let tags = msg.tags.clone().into_iter()
                        .filter_map(|(k, v)| v.map(|value| (k, value)))
                        .collect::<HashMap<String, String>>();
                    self.process_command(command, tags).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_cap(&mut self, subcommand: String, params: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        match subcommand.to_uppercase().as_str() {
            "LS" => {
                self.capabilities_negotiating = true;
                // Only advertise the core capabilities that our client actually supports
                let caps = vec![
                    "sasl",
                    "message-tags",
                    "server-time",
                    "batch",
                    "echo-message",
                    "+draft/react",
                    "+draft/reply",
                ];
                
                let cap_string = caps.join(" ");
                self.send_message(
                    Message::new("CAP")
                        .with_params(vec!["*".to_string(), "LS".to_string(), cap_string])
                ).await?;
            }
            "REQ" => {
                if let Some(requested) = params.first() {
                    let requested_caps: Vec<&str> = requested.split_whitespace().collect();
                    let mut ack_caps = Vec::new();
                    
                    for cap in requested_caps {
                        // Check if we support this capability
                        let cap_name = cap.split('=').next().unwrap_or(cap); // Handle capabilities with values
                        if ["sasl", "message-tags", "server-time", "batch", "echo-message", "+draft/react", "+draft/reply"].contains(&cap_name) {
                            ack_caps.push(cap);
                            self.capabilities_enabled.push(cap.to_string());
                        }
                    }
                    
                    if !ack_caps.is_empty() {
                        // Update capabilities in server state
                        {
                            let mut state = self.server_state.write().await;
                            if let Some(mut conn) = state.connections.get_mut(&self.id) {
                                conn.capabilities = self.capabilities_enabled.clone();
                            };
                        }
                        
                        self.send_message(
                            Message::new("CAP")
                                .with_params(vec!["*".to_string(), "ACK".to_string(), ack_caps.join(" ")])
                        ).await?;
                    } else {
                        self.send_message(
                            Message::new("CAP")
                                .with_params(vec!["*".to_string(), "NAK".to_string(), requested.clone()])
                        ).await?;
                    }
                }
            }
            "END" => {
                self.capabilities_negotiating = false;
                self.check_registration().await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_nick(&mut self, nick: String) -> Result<(), Box<dyn std::error::Error>> {
        // Validate nickname
        if !is_valid_nickname(&nick) {
            self.send_reply(Reply::ErroneousNickname {
                nick: "*".to_string(),
                attempted: nick,
            }).await?;
            return Ok(());
        }
        
        let nickname_available = {
            let state = self.server_state.write().await;
            state.is_nickname_available(&nick)
        };
        
        // Check if nickname is available
        if !nickname_available {
            self.send_reply(Reply::NicknameInUse {
                nick: "*".to_string(),
                attempted: nick,
            }).await?;
            return Ok(());
        }
        
        // Update connection info
        {
            let mut state = self.server_state.write().await;
            if let Some(mut conn) = state.connections.get_mut(&self.id) {
                // Unregister old nickname if any
                if let Some(old_nick) = &conn.nickname {
                    state.unregister_nickname(old_nick);
                }
                
                // Register new nickname
                if state.register_nickname(nick.clone(), self.id) {
                    conn.nickname = Some(nick);
                } else {
                    // Registration failed - nickname taken by someone else now
                    return Ok(());
                }
                drop(conn);
            }
            drop(state);
        }
        
        // Now check registration without holding the lock
        self.check_registration().await?;
        
        Ok(())
    }
    
    async fn handle_user(&mut self, username: String, realname: String) -> Result<(), Box<dyn std::error::Error>> {
        let conn_info = {
            let state = self.server_state.write().await;
            state.connections.get(&self.id).map(|conn| conn.username.is_some())
        };
        
        let already_registered = match conn_info {
            Some(registered) => registered,
            None => return Ok(()),
        };
        
        if already_registered {
            let nick = {
                let state = self.server_state.read().await;
                state.connections.get(&self.id)
                    .map(|conn| conn.nickname.clone().unwrap_or_else(|| "*".to_string()))
                    .unwrap_or_else(|| "*".to_string())
            };
            
            self.send_reply(Reply::AlreadyRegistered { nick }).await?;
            return Ok(());
        }
        
        // Update connection info
        {
            let mut state = self.server_state.write().await;
            if let Some(mut conn) = state.connections.get_mut(&self.id) {
                conn.username = Some(username);
                conn.realname = Some(realname);
                drop(conn);
            }
            drop(state);
        }
        
        self.check_registration().await?;
        
        Ok(())
    }
    
    async fn check_registration(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("check_registration: registered={}, capabilities_negotiating={}", self.registered, self.capabilities_negotiating);
        if self.registered || self.capabilities_negotiating {
            return Ok(());
        }
        
        let registration_info = {
            let state = self.server_state.read().await;
            state.connections.get(&self.id).and_then(|conn| {
                debug!("Connection state: nickname={:?}, username={:?}, registered={}", 
                       conn.nickname, conn.username, conn.registered);
                if conn.is_registered() {
                    debug!("Connection is registered, proceeding with welcome");
                    Some((conn.nickname.clone().unwrap(), state.server_name.clone()))
                } else {
                    debug!("Connection not fully registered yet");
                    None
                }
            })
        };
        
        let (nick, server_name) = match registration_info {
            Some(info) => info,
            None => return Ok(()),
        };
        
        self.registered = true;
        
        // Update registered status in server state
        {
            let state = self.server_state.write().await;
            if let Some(mut conn) = state.connections.get_mut(&self.id) {
                conn.registered = true;
            };
        }
                
                // Send welcome messages
                self.send_reply(Reply::Welcome {
                    nick: nick.clone(),
                    network: "IronChat".to_string(),
                }).await?;
                
                self.send_reply(Reply::YourHost {
                    nick: nick.clone(),
                    servername: server_name.clone(),
                    version: "ironchatd-0.1.0".to_string(),
                }).await?;
                
                self.send_reply(Reply::Created {
                    nick: nick.clone(),
                    date: "2025-01-01".to_string(),
                }).await?;
                
                self.send_reply(Reply::MyInfo {
                    nick: nick.clone(),
                    servername: server_name,
                    version: "ironchatd-0.1.0".to_string(),
                    usermodes: "aiwroOs".to_string(),
                    chanmodes: "beI,k,l,imnpst".to_string(),
                }).await?;
                
                // Send ISUPPORT
                self.send_reply(Reply::ISupport {
                    nick,
                    tokens: vec![
                        "CASEMAPPING=ascii".to_string(),
                        "CHANMODES=beI,k,l,imnpst".to_string(),
                        "CHANTYPES=#&".to_string(),
                        "MODES=12".to_string(),
                        "NICKLEN=30".to_string(),
                        "CHANNELLEN=50".to_string(),
                        "TOPICLEN=390".to_string(),
                        "KICKLEN=255".to_string(),
                        "AWAYLEN=255".to_string(),
                        "MAXTARGETS=4".to_string(),
                        "MAXLIST=beI:100".to_string(),
                        "MAXCHANNELS=50".to_string(),
                        "PREFIX=(qaohv)~&@%+".to_string(),
                        "STATUSMSG=~&@%+".to_string(),
                        "CALLERID=g".to_string(),
                        "DEAF=D".to_string(),
                        "KNOCK".to_string(),
                        "EXCEPTS=e".to_string(),
                        "INVEX=I".to_string(),
                        "CHANMODES=beI,k,l,imnpstCDGKNOPQRSTUVZ".to_string(),
                        "CHANLIMIT=#&:50".to_string(),
                        "IDCHAN=!:5".to_string(),
                        "SAFELIST".to_string(),
                        "WATCH=128".to_string(),
                        "MONITOR=128".to_string(),
                        "TARGMAX=NAMES:1,LIST:1,KICK:4,WHOIS:1,PRIVMSG:4,NOTICE:4,ACCEPT:,MONITOR:".to_string(),
                        "EXTBAN=$,arx".to_string(),
                        "CLIENTVER=3.0".to_string(),
                        "MSGREFTYPES=msgid,timestamp".to_string(), // For chathistory
                        "ACCOUNT-EXTBAN".to_string(), // 2024 account-extban support
                        "UTF8ONLY".to_string(), // UTF-8 only mode support
                        "BOT=B".to_string(), // Bot mode support
                    ],
                }).await?;
                
                // Send MOTD
                let motd_messages = crate::commands::handlers::motd::send_motd(
                    self.server_state.clone(),
                    self.id
                ).await?;
                
                for msg in motd_messages {
                    self.send_message(msg).await?;
                }
        
        Ok(())
    }
    
    async fn handle_ping(&mut self, token: String) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.server_state.read().await;
        let server_name = state.server_name.clone();
        drop(state);
        
        self.send_message(
            Message::new("PONG")
                .with_prefix(server_name)
                .with_params(vec![token])
        ).await
    }
    
    async fn handle_pong(&mut self, token: String) -> Result<(), Box<dyn std::error::Error>> {
        if self.ping_token.as_ref() == Some(&token) {
            self.ping_token = None;
        }
        Ok(())
    }
    
    async fn handle_quit(&mut self, reason: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let reason = reason.unwrap_or_else(|| "Client quit".to_string());
        info!("Client {} quit: {}", self.addr, reason);
        Ok(())
    }
    
    async fn process_command(&mut self, command: Command, msg_tags: HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        // Forward to command processor
        match command {
            Command::Join(channels, keys) => {
                // Handle JOIN command
                let responses = crate::commands::handlers::join::handle_join(
                    self.server_state.clone(),
                    self.id,
                    channels,
                    keys
                ).await?;
                
                for response in responses {
                    self.send_message(response).await?;
                }
            }
            Command::Part(channels, message) => {
                // Handle PART command
                let responses = crate::commands::handlers::part::handle_part(
                    self.server_state.clone(),
                    self.id,
                    channels,
                    message
                ).await?;
                
                for response in responses {
                    self.send_message(response).await?;
                }
            }
            Command::Privmsg { target, message } => {
                // Handle PRIVMSG command
                let responses = crate::commands::handlers::privmsg::handle_privmsg(
                    self.server_state.clone(),
                    self.id,
                    target,
                    message
                ).await?;
                
                for response in responses {
                    self.send_message(response).await?;
                }
            }
            Command::TagMsg { target } => {
                // Handle TAGMSG command (for reactions)
                if !self.capabilities_enabled.contains(&"+draft/react".to_string()) {
                    self.send_reply(Reply::UnknownCommand {
                        nick: self.get_nick().await,
                        command: "TAGMSG".to_string(),
                    }).await?;
                    return Ok(());
                }
                
                // Get the message tags from the incoming message
                let tags = msg_tags;
                
                // Forward the TAGMSG to the target
                let responses = crate::commands::handlers::tagmsg::handle_tagmsg(
                    self.server_state.clone(),
                    self.id,
                    target,
                    tags
                ).await?;
                
                for response in responses {
                    self.send_message(response).await?;
                }
            }
            Command::ChatHistory { subcommand, target, params } => {
                // Handle CHATHISTORY command
                let mut full_params = vec![subcommand, target];
                full_params.extend(params);
                let responses = crate::commands::handlers::chathistory::handle_chathistory(
                    self.server_state.clone(),
                    self.id,
                    full_params
                ).await?;
                
                for response in responses {
                    self.send_message(response).await?;
                }
            }
            Command::Whois(targets) => {
                let responses = crate::commands::handlers::whois::handle_whois(
                    self.server_state.clone(),
                    self.id,
                    targets
                ).await?;
                
                for response in responses {
                    self.send_message(response).await?;
                }
            }
            // ... handle other commands
            _ => {
                self.send_reply(Reply::UnknownCommand {
                    nick: self.get_nick().await,
                    command: format!("{:?}", command),
                }).await?;
            }
        }
        Ok(())
    }
    
    async fn send_ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.ping_token.is_some() {
            // Previous ping not answered
            return Err("Ping timeout".into());
        }
        
        let token = format!("{}", self.id);
        self.ping_token = Some(token.clone());
        
        let state = self.server_state.read().await;
        let server_name = state.server_name.clone();
        drop(state);
        
        self.send_message(
            Message::new("PING")
                .with_prefix(server_name)
                .with_params(vec![token])
        ).await
    }
    
    async fn send_message(&mut self, msg: Message) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(msg).await?;
        Ok(())
    }
    
    async fn send_reply(&mut self, reply: Reply) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.server_state.read().await;
        let server_name = state.server_name.clone();
        drop(state);
        
        let msg = reply.to_message(&server_name);
        self.send_message(msg).await
    }
    
    async fn send_error(&mut self, error: &str) {
        let _ = self.send_message(
            Message::new("ERROR")
                .with_params(vec![error.to_string()])
        ).await;
    }
    
    async fn get_nick(&self) -> String {
        let state = self.server_state.read().await;
        state.connections.get(&self.id)
            .and_then(|conn| conn.nickname.clone())
            .unwrap_or_else(|| "*".to_string())
    }
    
    async fn cleanup(&mut self) {
        let state = self.server_state.write().await;
        
        // Remove from channels
        // TODO: Implement channel cleanup
        
        // Unregister nickname
        if let Some(conn) = state.connections.get(&self.id) {
            if let Some(nick) = &conn.nickname {
                state.unregister_nickname(nick);
            }
        }
        
        // Remove connection
        state.connections.remove(&self.id);
    }
}

fn is_valid_nickname(nick: &str) -> bool {
    if nick.is_empty() || nick.len() > 30 {
        return false;
    }
    
    let first_char = nick.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' && first_char != '[' && 
       first_char != ']' && first_char != '{' && first_char != '}' && 
       first_char != '\\' && first_char != '|' {
        return false;
    }
    
    nick.chars().all(|c| {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '[' || c == ']' || 
        c == '{' || c == '}' || c == '\\' || c == '|' || c == '^' || c == '`'
    })
}