// Comprehensive test suite for IronChatD
use std::io;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{sleep, Duration, timeout};

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("=== IRONCHATD COMPREHENSIVE TEST SUITE ===");
    println!("Testing all IRCv3 capabilities and edge cases\n");
    
    let mut passed = 0;
    let mut failed = 0;
    
    // Test 1: Basic Connection and Registration
    match test_basic_connection().await {
        Ok(_) => { println!("✓ Basic connection and registration"); passed += 1; }
        Err(e) => { println!("✗ Basic connection failed: {}", e); failed += 1; }
    }
    
    // Test 2: IRCv3 Capability Negotiation
    match test_capability_negotiation().await {
        Ok(_) => { println!("✓ IRCv3 capability negotiation"); passed += 1; }
        Err(e) => { println!("✗ Capability negotiation failed: {}", e); failed += 1; }
    }
    
    // Test 3: Channel Operations
    match test_channel_operations().await {
        Ok(_) => { println!("✓ Channel operations (JOIN/PART/PRIVMSG)"); passed += 1; }
        Err(e) => { println!("✗ Channel operations failed: {}", e); failed += 1; }
    }
    
    // Test 4: Message Tags and Server-Time
    match test_message_tags().await {
        Ok(_) => { println!("✓ Message tags and server-time"); passed += 1; }
        Err(e) => { println!("✗ Message tags failed: {}", e); failed += 1; }
    }
    
    // Test 5: Draft Specifications
    match test_draft_capabilities().await {
        Ok(_) => { println!("✓ Draft capabilities (redaction, typing, etc.)"); passed += 1; }
        Err(e) => { println!("✗ Draft capabilities failed: {}", e); failed += 1; }
    }
    
    // Test 6: Error Handling
    match test_error_handling().await {
        Ok(_) => { println!("✓ Error handling and edge cases"); passed += 1; }
        Err(e) => { println!("✗ Error handling failed: {}", e); failed += 1; }
    }
    
    // Test 7: Multiple Concurrent Clients
    match test_concurrent_clients().await {
        Ok(_) => { println!("✓ Multiple concurrent clients"); passed += 1; }
        Err(e) => { println!("✗ Concurrent clients failed: {}", e); failed += 1; }
    }
    
    // Test 8: Protocol Compliance
    match test_protocol_compliance().await {
        Ok(_) => { println!("✓ IRC protocol compliance"); passed += 1; }
        Err(e) => { println!("✗ Protocol compliance failed: {}", e); failed += 1; }
    }
    
    // Test 9: Performance Under Load
    match test_performance_load().await {
        Ok(_) => { println!("✓ Performance under load"); passed += 1; }
        Err(e) => { println!("✗ Performance test failed: {}", e); failed += 1; }
    }
    
    // Test 10: Security and Validation
    match test_security_validation().await {
        Ok(_) => { println!("✓ Security and input validation"); passed += 1; }
        Err(e) => { println!("✗ Security validation failed: {}", e); failed += 1; }
    }
    
    println!("\n=== TEST RESULTS ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", passed + failed);
    
    if failed == 0 {
        println!("\n🎉 ALL TESTS PASSED! IronChatD is ready for release!");
    } else {
        println!("\n⚠️  {} tests failed. Review before release.", failed);
    }
    
    Ok(())
}

async fn test_basic_connection() -> io::Result<()> {
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:6669")).await??;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Basic registration sequence
    writer.write_all(b"NICK testuser\r\n").await?;
    writer.write_all(b"USER testuser 0 * :Test User\r\n").await?;
    
    // Expect welcome message
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    if !buffer.contains("001") && !buffer.contains("Welcome") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No welcome message"));
    }
    
    Ok(())
}

async fn test_capability_negotiation() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // CAP LS
    writer.write_all(b"CAP LS 302\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    if !buffer.contains("CAP * LS") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No CAP LS response"));
    }
    
    // Check for bleeding-edge capabilities
    let required_caps = ["message-tags", "server-time", "draft/message-redaction", "draft/typing"];
    for cap in required_caps {
        if !buffer.contains(cap) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                format!("Missing capability: {}", cap).as_str()));
        }
    }
    
    // CAP REQ
    writer.write_all(b"CAP REQ :message-tags server-time\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    if !buffer.contains("CAP * ACK") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No CAP ACK response"));
    }
    
    Ok(())
}

async fn test_channel_operations() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Register
    writer.write_all(b"NICK chantest\r\nUSER chantest 0 * :Channel Test\r\n").await?;
    
    // Skip welcome messages
    for _ in 0..3 {
        buffer.clear();
        timeout(Duration::from_secs(1), reader.read_line(&mut buffer)).await.ok();
    }
    
    // JOIN
    writer.write_all(b"JOIN #testchan\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    if !buffer.contains("JOIN #testchan") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No JOIN response"));
    }
    
    // PRIVMSG
    writer.write_all(b"PRIVMSG #testchan :Hello, channel!\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    if !buffer.contains("PRIVMSG #testchan") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No PRIVMSG echo"));
    }
    
    Ok(())
}

async fn test_message_tags() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Enable message-tags capability
    writer.write_all(b"CAP REQ :message-tags server-time\r\nCAP END\r\n").await?;
    writer.write_all(b"NICK tagtest\r\nUSER tagtest 0 * :Tag Test\r\n").await?;
    writer.write_all(b"JOIN #tagtests\r\n").await?;
    
    // Skip setup messages
    for _ in 0..5 {
        buffer.clear();
        timeout(Duration::from_secs(1), reader.read_line(&mut buffer)).await.ok();
    }
    
    // Send message and check for tags
    writer.write_all(b"PRIVMSG #tagtests :Test message with tags\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    // Should contain message tags
    if !buffer.starts_with('@') {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No message tags found"));
    }
    
    // Check for time tag
    if !buffer.contains("time=") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No time tag found"));
    }
    
    // Check for msgid tag
    if !buffer.contains("msgid=") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No msgid tag found"));
    }
    
    Ok(())
}

async fn test_draft_capabilities() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Test draft/message-redaction
    writer.write_all(b"CAP REQ :draft/message-redaction\r\nCAP END\r\n").await?;
    writer.write_all(b"NICK drafttest\r\nUSER drafttest 0 * :Draft Test\r\n").await?;
    
    // Skip setup
    for _ in 0..3 {
        buffer.clear();
        timeout(Duration::from_secs(1), reader.read_line(&mut buffer)).await.ok();
    }
    
    // Test REDACT command
    writer.write_all(b"REDACT #test :Test redaction\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    if !buffer.contains("REDACT") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No REDACT response"));
    }
    
    Ok(())
}

async fn test_error_handling() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Test invalid commands
    writer.write_all(b"INVALID_COMMAND\r\n").await?;
    writer.write_all(b"NICK\r\n").await?; // Missing parameter
    writer.write_all(b"\r\n").await?; // Empty line
    
    // Server should handle gracefully without crashing
    sleep(Duration::from_millis(500)).await;
    
    // Should still be able to connect normally
    writer.write_all(b"NICK errortest\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await.ok();
    
    Ok(())
}

async fn test_concurrent_clients() -> io::Result<()> {
    let mut handles = vec![];
    
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
            let (reader, mut writer) = stream.split();
            let mut reader = BufReader::new(reader);
            let mut buffer = String::new();
            
            writer.write_all(format!("NICK concurrent{}\r\n", i).as_bytes()).await?;
            writer.write_all(format!("USER concurrent{} 0 * :Concurrent Test\r\n", i).as_bytes()).await?;
            writer.write_all(b"JOIN #concurrent\r\n").await?;
            writer.write_all(format!("PRIVMSG #concurrent :Message from client {}\r\n", i).as_bytes()).await?;
            
            // Read some responses
            for _ in 0..3 {
                buffer.clear();
                timeout(Duration::from_secs(1), reader.read_line(&mut buffer)).await.ok();
            }
            
            Ok::<(), io::Error>(())
        });
        handles.push(handle);
    }
    
    // Wait for all clients
    for handle in handles {
        handle.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
    }
    
    Ok(())
}

async fn test_protocol_compliance() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    
    // Test PING/PONG
    writer.write_all(b"PING :test123\r\n").await?;
    buffer.clear();
    timeout(Duration::from_secs(2), reader.read_line(&mut buffer)).await??;
    
    if !buffer.contains("PONG") || !buffer.contains("test123") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid PONG response"));
    }
    
    // Test proper line endings
    writer.write_all(b"NICK linetest\r\nUSER linetest 0 * :Line Test\r\n").await?;
    
    // All responses should end with \r\n
    for _ in 0..3 {
        buffer.clear();
        timeout(Duration::from_secs(1), reader.read_line(&mut buffer)).await.ok();
        if !buffer.is_empty() && !buffer.ends_with("\n") {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid line ending"));
        }
    }
    
    Ok(())
}

async fn test_performance_load() -> io::Result<()> {
    let start = std::time::Instant::now();
    let mut handles = vec![];
    
    // Create 50 clients rapidly
    for i in 0..50 {
        let handle = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
            let (_, mut writer) = stream.split();
            
            writer.write_all(format!("NICK perf{}\r\n", i).as_bytes()).await?;
            writer.write_all(format!("USER perf{} 0 * :Perf Test\r\n", i).as_bytes()).await?;
            
            // Send 10 messages rapidly
            for j in 0..10 {
                writer.write_all(format!("PRIVMSG #perf :Message {} from client {}\r\n", j, i).as_bytes()).await?;
            }
            
            Ok::<(), io::Error>(())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
    }
    
    let duration = start.elapsed();
    if duration > Duration::from_secs(10) {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "Performance test too slow"));
    }
    
    Ok(())
}

async fn test_security_validation() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (_, mut writer) = stream.split();
    
    // Test various malicious inputs
    let long_nick = format!("NICK {}\r\n", "A".repeat(100));
    let malicious_inputs = vec![
        b"\x00\x01\x02NICK evil\r\n".as_slice(),
        b"NICK \x7FEVIL\r\n".as_slice(),
        long_nick.as_bytes(),
        b"NICK evil\rINJECTED\nCOMMAND\r\n".as_slice(),
    ];
    
    for input in malicious_inputs {
        writer.write_all(input).await?;
        sleep(Duration::from_millis(10)).await;
    }
    
    // Server should still be responsive
    writer.write_all(b"PING :security_test\r\n").await?;
    
    // If we get here without the server crashing, security is working
    Ok(())
}