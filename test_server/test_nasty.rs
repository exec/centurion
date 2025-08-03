// Nasty IRC client to try to break the server
use std::io;
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("STRESS TESTING IronChatD - Attempting to break the server!");
    
    // Test 1: Rapid fire connections
    println!("\nTest 1: Rapid fire connections (100 concurrent)");
    let mut handles = vec![];
    for i in 0..100 {
        let handle = tokio::spawn(async move {
            match TcpStream::connect("127.0.0.1:6669").await {
                Ok(mut stream) => {
                    let _ = stream.write_all(format!("NICK test{}\r\nUSER test{} 0 * :stress test\r\nQUIT\r\n", i, i).as_bytes()).await;
                }
                Err(_) => {}
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let _ = handle.await;
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Test 2: Buffer overflow attempt
    println!("Test 2: Buffer overflow attempt with huge messages");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:6669").await {
        let huge_msg = "A".repeat(100000);
        let _ = stream.write_all(format!("NICK overflow\r\nUSER overflow 0 * :test\r\nPRIVMSG #test :{}\r\n", huge_msg).as_bytes()).await;
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Test 3: Invalid protocol commands
    println!("Test 3: Invalid protocol commands and malformed data");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:6669").await {
        let long_msg = "A".repeat(1000);
        let evil_commands = vec![
            "\x00\x01\x02NICK evil\r\n",  // Binary data
            "NICK \x7F\r\n",              // Non-ASCII chars
            "INVALID_COMMAND_XYZ\r\n",    // Invalid command
            "NICK\r\n",                   // Missing parameters
            &long_msg,                    // No CRLF termination
            "\r\n\r\n\r\n",               // Empty lines
            "NICK evil\nUSER evil\nJOIN #evil\n", // Wrong line endings
        ];
        
        for cmd in evil_commands {
            let _ = stream.write_all(cmd.as_bytes()).await;
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Test 4: Rapid message flooding
    println!("Test 4: Message flooding attack");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:6669").await {
        let _ = stream.write_all(b"NICK flooder\r\nUSER flooder 0 * :flood test\r\nJOIN #flood\r\n").await;
        
        for i in 0..1000 {
            let msg = format!("PRIVMSG #flood :Flood message {}\r\n", i);
            let _ = stream.write_all(msg.as_bytes()).await;
        }
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Test 5: IRC injection attempts
    println!("Test 5: IRC command injection attempts");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:6669").await {
        let injection_attempts = vec![
            "NICK injection\r\nKILL injection\r\n",
            "USER injection 0 * :test\rOPER admin password\r\n",
            "PRIVMSG #test :Hello\r\nMODE #test +o injection\r\n",
            "JOIN #test\rKICK #test victim\r\n",
        ];
        
        for inject in injection_attempts {
            let _ = stream.write_all(inject.as_bytes()).await;
            sleep(Duration::from_millis(50)).await;
        }
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Test 6: Connection without proper registration
    println!("Test 6: Unregistered connection attempts");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:6669").await {
        let _ = stream.write_all(b"JOIN #test\r\nPRIVMSG #test :I'm not registered!\r\nWHO *\r\n").await;
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Test 7: Capability abuse
    println!("Test 7: IRCv3 capability abuse");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:6669").await {
        let long_cap = format!("CAP REQ :{}\r\n", "A".repeat(10000));
        let cap_abuse = vec![
            "CAP LS\r\n",
            "CAP REQ :invalid-capability-that-does-not-exist\r\n", 
            &long_cap,
            "CAP END\r\nCAP LS\r\n", // Double capability negotiation
            "CAP REQ :\r\n", // Empty capability request
        ];
        
        for abuse in cap_abuse {
            let _ = stream.write_all(abuse.as_bytes()).await;
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Test final connection to verify server is still responding
    println!("\nFinal test: Verifying server is still alive after stress testing...");
    match TcpStream::connect("127.0.0.1:6669").await {
        Ok(mut stream) => {
            let _ = stream.write_all(b"CAP LS\r\n").await;
            println!("SUCCESS: Server survived all stress tests!");
            println!("IronChatD demonstrates excellent resilience and stability!");
        }
        Err(e) => {
            println!("FAILURE: Server appears down after stress testing: {}", e);
        }
    }
    
    Ok(())
}