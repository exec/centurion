// Simple IRC server test to demonstrate the core functionality
use std::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("🦾 IronChatD - Bleeding Edge IRCv3 Server Starting on 127.0.0.1:6669");
    
    let listener = TcpListener::bind("127.0.0.1:6669").await?;
    println!("Server listening on 127.0.0.1:6669");
    
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from: {}", addr);
        
        tokio::spawn(async move {
            handle_client(socket).await.unwrap_or_else(|e| {
                println!("Error handling client {}: {}", addr, e);
            });
        });
    }
}

async fn handle_client(mut socket: TcpStream) -> io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Send IRCv3 capabilities announcement
    writer.write_all(b"CAP * LS :account-notify away-notify cap-notify chghost extended-join invite-notify multi-prefix sasl tls userhost-in-names message-tags batch server-time echo-message labeled-response draft/chathistory draft/event-playback draft/message-redaction draft/account-extban draft/metadata-2 draft/multiline draft/read-marker draft/relaymsg draft/typing draft/pre-away\r\n").await?;
    
    loop {
        buffer.clear();
        
        match reader.read_line(&mut buffer).await {
            Ok(0) => break, // Connection closed
            Ok(_) => {
                let line = buffer.trim();
                println!("Received: {}", line);
                
                // Handle basic IRC commands
                if line.starts_with("CAP LS") {
                    writer.write_all(b"CAP * LS :account-notify away-notify cap-notify chghost extended-join invite-notify multi-prefix sasl tls userhost-in-names message-tags batch server-time echo-message labeled-response draft/chathistory draft/event-playback draft/message-redaction draft/account-extban draft/metadata-2 draft/multiline draft/read-marker draft/relaymsg draft/typing draft/pre-away\r\n").await?;
                } else if line.starts_with("CAP REQ") {
                    // Accept all capability requests for demo
                    let caps = line.split_once(" :").map(|(_, caps)| caps).unwrap_or("");
                    let response = format!("CAP * ACK :{}\r\n", caps);
                    writer.write_all(response.as_bytes()).await?;
                } else if line.starts_with("CAP END") {
                    writer.write_all(b"001 testuser :Welcome to IronChatD IRCv3 Server\r\n").await?;
                    writer.write_all(b"002 testuser :Your host is ironchatd.local, running version ironchatd-0.1.0\r\n").await?;
                    writer.write_all(b"003 testuser :This server was created 2025-01-01\r\n").await?;
                    writer.write_all(b"004 testuser ironchatd.local ironchatd-0.1.0 aiwroOs beI,k,l,imnpst\r\n").await?;
                    writer.write_all(b"005 testuser CASEMAPPING=ascii CHANTYPES=# PREFIX=(ov)@+ NETWORK=IronChat :are supported by this server\r\n").await?;
                } else if line.starts_with("NICK") {
                    writer.write_all(b"001 testuser :Welcome to IronChatD - Bleeding Edge IRCv3 Server\r\n").await?;
                } else if line.starts_with("USER") {
                    // User registration complete
                } else if line.starts_with("JOIN") {
                    let channel = line.split_whitespace().nth(1).unwrap_or("#test");
                    let response = format!(":{} JOIN {}\r\n", "testuser!user@host", channel);
                    writer.write_all(response.as_bytes()).await?;
                    let topic_response = format!("332 testuser {} :Welcome to IronChatD - Testing bleeding-edge IRCv3 capabilities!\r\n", channel);
                    writer.write_all(topic_response.as_bytes()).await?;
                } else if line.starts_with("PRIVMSG") {
                    // Echo back with bleeding-edge features demo
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let target = parts[1];
                        let message = line.split_once(" :").map(|(_, msg)| msg).unwrap_or("Hello");
                        
                        // Demonstrate bleeding-edge IRCv3 features with message tags
                        let response = format!("@time={};msgid={};+example-tag=demo :testuser!user@host PRIVMSG {} :Echo: {}\r\n", 
                            chrono::Utc::now().to_rfc3339(), 
                            uuid::Uuid::new_v4(),
                            target, 
                            message
                        );
                        writer.write_all(response.as_bytes()).await?;
                    }
                } else if line.starts_with("PING") {
                    writer.write_all(b"PONG :ironchatd.local\r\n").await?;
                } else if line.starts_with("REDACT") {
                    // Demo message redaction capability
                    writer.write_all(b"@time=2025-01-01T00:00:00Z :ironchatd.local REDACT testuser :Message redacted successfully\r\n").await?;
                }
            }
            Err(e) => {
                println!("Error reading from socket: {}", e);
                break;
            }
        }
    }
    
    println!("Client disconnected");
    Ok(())
}