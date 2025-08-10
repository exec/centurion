use super::*;
use crate::state::{ServerState, Connection, Channel};
use crate::protocol::{Message, Reply};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

async fn setup_test_state() -> (Arc<RwLock<ServerState>>, u64, u64, u64) {
    let state = Arc::new(RwLock::new(ServerState::new()));
    
    let (tx1, _) = mpsc::channel(100);
    let (tx2, _) = mpsc::channel(100);
    let (tx3, _) = mpsc::channel(100);
    
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
    
    let requester_id = 1;
    let member_id = 2;
    let other_id = 3;
    
    let mut state_guard = state.write().await;
    
    // Add connections
    let mut requester = Connection::new(requester_id, addr, tx1);
    requester.nickname = Some("requester".to_string());
    requester.username = Some("requester".to_string());
    requester.hostname = "test.host".to_string();
    requester.registered = true;
    state_guard.connections.insert(requester_id, requester);
    state_guard.nicknames.insert("requester".to_string(), requester_id);
    
    let mut member = Connection::new(member_id, addr, tx2);
    member.nickname = Some("member".to_string());
    member.username = Some("member".to_string());
    member.hostname = "member.host".to_string();
    member.registered = true;
    state_guard.connections.insert(member_id, member);
    state_guard.nicknames.insert("member".to_string(), member_id);
    
    let mut other = Connection::new(other_id, addr, tx3);
    other.nickname = Some("other".to_string());
    other.username = Some("other".to_string());
    other.hostname = "other.host".to_string();
    other.registered = true;
    state_guard.connections.insert(other_id, other);
    state_guard.nicknames.insert("other".to_string(), other_id);
    
    // Create test channels
    let channel = Channel::new("#test".to_string());
    channel.add_member(requester_id, true); // operator
    channel.add_member(member_id, false);
    state_guard.channels.insert("#test".to_string(), channel);
    
    // Create a secret channel
    let mut secret_channel = Channel::new("#secret".to_string());
    secret_channel.add_member(member_id, true);
    secret_channel.modes.push('s'); // secret mode
    state_guard.channels.insert("#secret".to_string(), secret_channel);
    
    // Create a private channel
    let mut private_channel = Channel::new("#private".to_string());
    private_channel.add_member(other_id, true);
    private_channel.modes.push('p'); // private mode
    state_guard.channels.insert("#private".to_string(), private_channel);
    
    drop(state_guard);
    
    (state, requester_id, member_id, other_id)
}

#[tokio::test]
async fn test_who_no_params() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec![];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO for "*" (all users)
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "*");
    assert_eq!(messages[0].params[2], "End of WHO list");
}

#[tokio::test]
async fn test_who_channel_member() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO for channel
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "#test");
    assert_eq!(messages[0].params[2], "End of WHO list");
}

#[tokio::test]
async fn test_who_channel_not_member() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["#other".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO for channel (even if not member of non-existent channel)
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "#other");
}

#[tokio::test]
async fn test_who_secret_channel_not_member() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["#secret".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO but no member info since not in secret channel
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "#secret");
}

#[tokio::test]
async fn test_who_private_channel_not_member() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["#private".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO but no member info since not in private channel
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "#private");
}

#[tokio::test]
async fn test_who_specific_user_exists() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["member".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO for specific user (no error since user exists)
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "member");
}

#[tokio::test]
async fn test_who_specific_user_not_exists() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["nonexistent".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 2);
    
    // Should get NoSuchNick error first
    assert!(messages[0].to_string().contains("401")); // ERR_NOSUCHNICK
    assert!(messages[0].to_string().contains("nonexistent"));
    
    // Then end of WHO
    assert_eq!(messages[1].command, "315");
    assert_eq!(messages[1].params[1], "nonexistent");
}

#[tokio::test]
async fn test_who_wildcard() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["*".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get end of WHO for wildcard
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "*");
    assert_eq!(messages[0].params[2], "End of WHO list");
}

#[tokio::test]
async fn test_who_case_insensitive_nick() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    let params = vec!["MEMBER".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should find user with case-insensitive lookup
    assert_eq!(messages[0].command, "315");
    assert_eq!(messages[0].params[1], "MEMBER");
}

#[tokio::test]
async fn test_who_invalid_connection() {
    let state = Arc::new(RwLock::new(ServerState::new()));
    let invalid_id = 999;
    
    let params = vec!["#test".to_string()];
    let result = handle_who(state, invalid_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0); // No messages when connection doesn't exist
}

#[tokio::test]
async fn test_who_channel_variations() {
    let (state, requester_id, _, _) = setup_test_state().await;
    
    // Test different channel prefixes
    let test_cases = vec![
        "#channel",  // Standard channel
        "&channel",  // Local channel
        "!channel",  // Safe channel
    ];
    
    for channel_name in test_cases {
        let params = vec![channel_name.to_string()];
        let result = handle_who(state.clone(), requester_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        
        // Should get end of WHO for each channel type
        assert_eq!(messages[0].command, "315");
        assert_eq!(messages[0].params[1], channel_name);
    }
}

#[tokio::test]
async fn test_who_requester_without_nickname() {
    let state = Arc::new(RwLock::new(ServerState::new()));
    let requester_id = 1;
    
    // Create connection without nickname
    let (tx, _) = mpsc::channel(100);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
    let connection = Connection::new(requester_id, addr, tx);
    
    {
        let mut state_guard = state.write().await;
        state_guard.connections.insert(requester_id, connection);
    }
    
    let params = vec!["test".to_string()];
    let result = handle_who(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 2);
    
    // Should use "*" as nick for connection without nickname
    assert!(messages[0].to_string().contains("*")); // NoSuchNick with "*" nick
    assert_eq!(messages[1].params[0], "*"); // End of WHO with "*" nick
}