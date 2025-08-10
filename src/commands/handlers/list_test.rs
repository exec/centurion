use super::*;
use crate::state::{ServerState, Connection, Channel};
use crate::protocol::{Message, Reply};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

async fn setup_test_state() -> (Arc<RwLock<ServerState>>, u64, u64) {
    let state = Arc::new(RwLock::new(ServerState::new()));
    
    let (tx1, _) = mpsc::channel(100);
    let (tx2, _) = mpsc::channel(100);
    
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
    
    let requester_id = 1;
    let member_id = 2;
    
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
    
    // Create test channels
    let mut public_channel = Channel::new("#public".to_string());
    public_channel.add_member(requester_id, false);
    public_channel.add_member(member_id, false);
    public_channel.topic = Some("Public channel topic".to_string());
    state_guard.channels.insert("#public".to_string(), public_channel);
    
    let mut secret_channel = Channel::new("#secret".to_string());
    secret_channel.add_member(member_id, true);
    secret_channel.modes.push('s'); // secret mode
    secret_channel.topic = Some("Secret channel topic".to_string());
    state_guard.channels.insert("#secret".to_string(), secret_channel);
    
    let mut private_channel = Channel::new("#private".to_string());
    private_channel.add_member(member_id, true);
    private_channel.modes.push('p'); // private mode
    private_channel.topic = Some("Private channel topic".to_string());
    state_guard.channels.insert("#private".to_string(), private_channel);
    
    let mut empty_channel = Channel::new("#empty".to_string());
    state_guard.channels.insert("#empty".to_string(), empty_channel);
    
    let mut no_topic_channel = Channel::new("#notopic".to_string());
    no_topic_channel.add_member(requester_id, false);
    state_guard.channels.insert("#notopic".to_string(), no_topic_channel);
    
    drop(state_guard);
    
    (state, requester_id, member_id)
}

#[tokio::test]
async fn test_list_all_channels() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec![];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + visible channels (322) + end (323)
    // Visible channels: #public, #empty, #notopic (requester is member of #public and #notopic)
    assert!(messages.len() >= 3); // At least start + end + some channels
    
    // Check list start
    assert_eq!(messages[0].command, "321");
    assert_eq!(messages[0].params[0], "requester");
    
    // Check list end
    let last_msg = messages.last().unwrap();
    assert_eq!(last_msg.command, "323");
    assert_eq!(last_msg.params[0], "requester");
    assert_eq!(last_msg.params[1], "End of LIST");
    
    // Check that we have channel listings
    let channel_messages: Vec<_> = messages.iter()
        .filter(|msg| msg.command == "322")
        .collect();
    
    // Should see at least public channels
    assert!(!channel_messages.is_empty());
    
    // Check for public channel
    let public_listing = channel_messages.iter()
        .find(|msg| msg.params[1] == "#public");
    assert!(public_listing.is_some());
    let public_msg = public_listing.unwrap();
    assert_eq!(public_msg.params[2], "2"); // 2 members
    assert_eq!(public_msg.params[3], "Public channel topic");
}

#[tokio::test]
async fn test_list_specific_channel() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#public".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + channel (322) + end (323)
    assert_eq!(messages.len(), 3);
    
    // Check list start
    assert_eq!(messages[0].command, "321");
    
    // Check channel listing
    assert_eq!(messages[1].command, "322");
    assert_eq!(messages[1].params[0], "requester");
    assert_eq!(messages[1].params[1], "#public");
    assert_eq!(messages[1].params[2], "2"); // 2 members
    assert_eq!(messages[1].params[3], "Public channel topic");
    
    // Check list end
    assert_eq!(messages[2].command, "323");
}

#[tokio::test]
async fn test_list_multiple_channels() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#public,#notopic".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + 2 channels (322) + end (323)
    assert_eq!(messages.len(), 4);
    
    // Check list start
    assert_eq!(messages[0].command, "321");
    
    // Check channel listings
    let channel_names: Vec<String> = messages.iter()
        .filter(|msg| msg.command == "322")
        .map(|msg| msg.params[1].clone())
        .collect();
    
    assert!(channel_names.contains(&"#public".to_string()));
    assert!(channel_names.contains(&"#notopic".to_string()));
    
    // Check list end
    let last_msg = messages.last().unwrap();
    assert_eq!(last_msg.command, "323");
}

#[tokio::test]
async fn test_list_nonexistent_channel() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#nonexistent".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + end (323) only (no channel listing)
    assert_eq!(messages.len(), 2);
    
    assert_eq!(messages[0].command, "321");
    assert_eq!(messages[1].command, "323");
    
    // No 322 messages for nonexistent channels
    let channel_listings = messages.iter()
        .filter(|msg| msg.command == "322")
        .count();
    assert_eq!(channel_listings, 0);
}

#[tokio::test]
async fn test_list_secret_channel_not_member() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#secret".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + end (323) only (secret channel not visible)
    assert_eq!(messages.len(), 2);
    
    assert_eq!(messages[0].command, "321");
    assert_eq!(messages[1].command, "323");
    
    // No channel listing for secret channel when not a member
    let secret_listings = messages.iter()
        .filter(|msg| msg.command == "322" && msg.params[1] == "#secret")
        .count();
    assert_eq!(secret_listings, 0);
}

#[tokio::test]
async fn test_list_secret_channel_as_member() {
    let (state, _, member_id) = setup_test_state().await;
    
    let params = vec!["#secret".to_string()];
    let result = handle_list(state, member_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + channel (322) + end (323)
    assert_eq!(messages.len(), 3);
    
    // Check channel listing is visible to member
    assert_eq!(messages[1].command, "322");
    assert_eq!(messages[1].params[1], "#secret");
    assert_eq!(messages[1].params[2], "1"); // 1 member
    assert_eq!(messages[1].params[3], "Secret channel topic");
}

#[tokio::test]
async fn test_list_private_channel_not_member() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#private".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + end (323) only (private channel not visible)
    assert_eq!(messages.len(), 2);
    
    assert_eq!(messages[0].command, "321");
    assert_eq!(messages[1].command, "323");
    
    // No channel listing for private channel when not a member
    let private_listings = messages.iter()
        .filter(|msg| msg.command == "322" && msg.params[1] == "#private")
        .count();
    assert_eq!(private_listings, 0);
}

#[tokio::test]
async fn test_list_private_channel_as_member() {
    let (state, _, member_id) = setup_test_state().await;
    
    let params = vec!["#private".to_string()];
    let result = handle_list(state, member_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + channel (322) + end (323)
    assert_eq!(messages.len(), 3);
    
    // Check channel listing is visible to member
    assert_eq!(messages[1].command, "322");
    assert_eq!(messages[1].params[1], "#private");
    assert_eq!(messages[1].params[2], "1"); // 1 member
    assert_eq!(messages[1].params[3], "Private channel topic");
}

#[tokio::test]
async fn test_list_channel_with_no_topic() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#notopic".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + channel (322) + end (323)
    assert_eq!(messages.len(), 3);
    
    // Check channel listing has empty topic
    assert_eq!(messages[1].command, "322");
    assert_eq!(messages[1].params[1], "#notopic");
    assert_eq!(messages[1].params[2], "1"); // 1 member
    assert_eq!(messages[1].params[3], ""); // Empty topic
}

#[tokio::test]
async fn test_list_empty_channel() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#empty".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should have: start (321) + channel (322) + end (323)
    assert_eq!(messages.len(), 3);
    
    // Check channel listing shows 0 members
    assert_eq!(messages[1].command, "322");
    assert_eq!(messages[1].params[1], "#empty");
    assert_eq!(messages[1].params[2], "0"); // 0 members
    assert_eq!(messages[1].params[3], ""); // No topic
}

#[tokio::test]
async fn test_list_invalid_connection() {
    let state = Arc::new(RwLock::new(ServerState::new()));
    let invalid_id = 999;
    
    let params = vec![];
    let result = handle_list(state, invalid_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0); // No messages when connection doesn't exist
}

#[tokio::test]
async fn test_list_requester_without_nickname() {
    let state = Arc::new(RwLock::new(ServerState::new()));
    let requester_id = 1;
    
    // Create connection without nickname
    let (tx, _) = mpsc::channel(100);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
    let connection = Connection::new(requester_id, addr, tx);
    
    // Add a test channel
    let channel = Channel::new("#test".to_string());
    
    {
        let mut state_guard = state.write().await;
        state_guard.connections.insert(requester_id, connection);
        state_guard.channels.insert("#test".to_string(), channel);
    }
    
    let params = vec![];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should use "*" as nick for connection without nickname
    assert!(messages.len() >= 2); // At least start and end
    assert_eq!(messages[0].params[0], "*"); // Start message with "*" nick
    
    let last_msg = messages.last().unwrap();
    assert_eq!(last_msg.params[0], "*"); // End message with "*" nick
}

#[tokio::test]
async fn test_list_mixed_channel_visibility() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec!["#public,#secret,#private,#notopic".to_string()];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should show only channels requester can see (#public and #notopic)
    let channel_listings: Vec<_> = messages.iter()
        .filter(|msg| msg.command == "322")
        .collect();
    
    assert_eq!(channel_listings.len(), 2);
    
    let channel_names: Vec<String> = channel_listings.iter()
        .map(|msg| msg.params[1].clone())
        .collect();
    
    assert!(channel_names.contains(&"#public".to_string()));
    assert!(channel_names.contains(&"#notopic".to_string()));
    assert!(!channel_names.contains(&"#secret".to_string()));
    assert!(!channel_names.contains(&"#private".to_string()));
}

#[tokio::test]
async fn test_list_all_excludes_hidden_channels() {
    let (state, requester_id, _) = setup_test_state().await;
    
    let params = vec![];
    let result = handle_list(state, requester_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    let channel_listings: Vec<_> = messages.iter()
        .filter(|msg| msg.command == "322")
        .collect();
    
    let channel_names: Vec<String> = channel_listings.iter()
        .map(|msg| msg.params[1].clone())
        .collect();
    
    // Should not see secret or private channels where not a member
    assert!(!channel_names.contains(&"#secret".to_string()));
    assert!(!channel_names.contains(&"#private".to_string()));
    
    // Should see public channels
    assert!(channel_names.contains(&"#public".to_string()));
    assert!(channel_names.contains(&"#notopic".to_string()));
    assert!(channel_names.contains(&"#empty".to_string()));
}