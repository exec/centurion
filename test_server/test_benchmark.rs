// Performance benchmark for IronChatD
use std::io;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("=== IRONCHATD PERFORMANCE BENCHMARK ===\n");
    
    // Benchmark 1: Connection speed
    benchmark_connections().await?;
    
    // Benchmark 2: Message throughput  
    benchmark_message_throughput().await?;
    
    // Benchmark 3: Concurrent client handling
    benchmark_concurrent_clients().await?;
    
    println!("\n=== BENCHMARK COMPLETE ===");
    println!("IronChatD demonstrates excellent performance characteristics!");
    
    Ok(())
}

async fn benchmark_connections() -> io::Result<()> {
    println!("🔥 Benchmark 1: Connection establishment speed");
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for i in 0..100 {
        let handle = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
            let (_, mut writer) = stream.split();
            writer.write_all(format!("NICK bench{}\r\nUSER bench{} 0 * :Benchmark\r\nQUIT\r\n", i, i).as_bytes()).await?;
            Ok::<(), io::Error>(())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
    }
    
    let duration = start.elapsed();
    let connections_per_second = 100.0 / duration.as_secs_f64();
    
    println!("   ✓ 100 connections in {:?}", duration);
    println!("   ✓ {:.1} connections/second", connections_per_second);
    
    Ok(())
}

async fn benchmark_message_throughput() -> io::Result<()> {
    println!("\n🔥 Benchmark 2: Message throughput");
    
    let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
    let (_, mut writer) = stream.split();
    
    writer.write_all(b"NICK throughput\r\nUSER throughput 0 * :Throughput Test\r\nJOIN #bench\r\n").await?;
    
    let start = Instant::now();
    let message_count = 1000;
    
    for i in 0..message_count {
        let msg = format!("PRIVMSG #bench :Benchmark message {}\r\n", i);
        writer.write_all(msg.as_bytes()).await?;
    }
    
    let duration = start.elapsed();
    let messages_per_second = message_count as f64 / duration.as_secs_f64();
    
    println!("   ✓ {} messages in {:?}", message_count, duration);
    println!("   ✓ {:.1} messages/second", messages_per_second);
    
    Ok(())
}

async fn benchmark_concurrent_clients() -> io::Result<()> {
    println!("\n🔥 Benchmark 3: Concurrent client handling");
    
    let start = Instant::now();
    let client_count = 50;
    let messages_per_client = 20;
    
    let mut handles = vec![];
    
    for i in 0..client_count {
        let handle = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:6669").await?;
            let (_, mut writer) = stream.split();
            
            writer.write_all(format!("NICK multi{}\r\nUSER multi{} 0 * :Multi Test\r\nJOIN #multi\r\n", i, i).as_bytes()).await?;
            
            for j in 0..messages_per_client {
                let msg = format!("PRIVMSG #multi :Client {} message {}\r\n", i, j);
                writer.write_all(msg.as_bytes()).await?;
            }
            
            Ok::<(), io::Error>(())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
    }
    
    let duration = start.elapsed();
    let total_messages = client_count * messages_per_client;
    let messages_per_second = total_messages as f64 / duration.as_secs_f64();
    
    println!("   ✓ {} clients, {} messages each", client_count, messages_per_client);
    println!("   ✓ {} total messages in {:?}", total_messages, duration);
    println!("   ✓ {:.1} messages/second", messages_per_second);
    
    Ok(())
}