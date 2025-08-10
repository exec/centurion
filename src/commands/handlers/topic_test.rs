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
    
    let user_id = 1;
    let member_id = 2;
    
    let mut state_guard = state.write().await;
    
    // Add connections
    let mut user = Connection::new(user_id, addr, tx1);
    user.nickname = Some("user".to_string());
    user.username = Some("user".to_string());
    user.registered = true;
    state_guard.connections.insert(user_id, user);
    state_guard.nicknames.insert("user".to_string(), user_id);
    
    let mut member = Connection::new(member_id, addr, tx2);
    member.nickname = Some("member".to_string());
    member.username = Some("member".to_string());
    member.registered = true;
    state_guard.connections.insert(member_id, member);
    state_guard.nicknames.insert("member".to_string(), member_id);
    
    // Create test channel
    let channel = Channel::new("#test".to_string());
    channel.add_member(user_id, false);
    channel.add_member(member_id, false);
    
    state_guard.channels.insert("#test".to_string(), channel);
    
    drop(state_guard);
    
    (state, user_id, member_id)
}

#[tokio::test]
async fn test_topic_get_no_topic() {
    let (state, user_id, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string()];
    let result = handle_topic(state, user_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NoTopic reply
    assert!(messages[0].to_string().contains("331")); // RPL_NOTOPIC
}

#[tokio::test]
async fn test_topic_set_success() {
    let (state, user_id, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "This is a test topic".to_string()];
    let result = handle_topic(state.clone(), user_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert!(!messages.is_empty());
    
    // Verify topic was set
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.topic, Some("This is a test topic".to_string()));
    assert!(channel.topic_set_by.is_some());
    assert!(channel.topic_set_at.is_some());
}

#[tokio::test]
async fn test_topic_get_existing() {
    let (state, user_id, _) = setup_test_state().await;
    
    // First set a topic
    let set_params = vec!["#test".to_string(), "Existing topic".to_string()];
    handle_topic(state.clone(), user_id, set_params).await.unwrap();
    
    // Then get the topic
    let get_params = vec!["#test".to_string()];
    let result = handle_topic(state, user_id, get_params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1); // Only Topic reply, no TopicWhoTime since it's not implemented
    
    // Should get Topic reply
    assert!(messages[0].to_string().contains("332")); // RPL_TOPIC
}

#[tokio::test]
async fn test_topic_clear() {
    let (state, user_id, _) = setup_test_state().await;
    
    // First set a topic
    let set_params = vec!["#test".to_string(), "Topic to clear".to_string()];
    handle_topic(state.clone(), user_id, set_params).await.unwrap();
    
    // Then clear it with empty string
    let clear_params = vec!["#test".to_string(), "".to_string()];
    let result = handle_topic(state.clone(), user_id, clear_params).await;
    
    assert!(result.is_ok());
    
    // Verify topic was cleared
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.topic, None);
}

#[tokio::test]
async fn test_topic_restricted_mode() {
    let (state, user_id, member_id) = setup_test_state().await;
    
    // Set channel to +t mode (topic restricted)
    {
        let state_guard = state.write().await;
        let mut channel = state_guard.channels.get_mut("#test").unwrap();
        channel.modes.push('t');
    }
    
    // Non-operator should not be able to set topic
    let params = vec!["#test".to_string(), "Restricted topic".to_string()];
    let result = handle_topic(state.clone(), member_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get ChanOpPrivsNeeded error
    assert!(messages[0].to_string().contains("482")); // ERR_CHANOPRIVSNEEDED
}

#[tokio::test]
async fn test_topic_operator_restricted_mode() {
    let (state, user_id, _) = setup_test_state().await;
    
    // Make user an operator and set channel to +t mode
    {
        let state_guard = state.write().await;
        let mut channel = state_guard.channels.get_mut("#test").unwrap();
        channel.modes.push('t');
        // Remove user and re-add as operator
        channel.members.remove(&user_id);
        channel.add_member(user_id, true); // true = is operator
    }
    
    // Operator should be able to set topic
    let params = vec!["#test".to_string(), "Operator topic".to_string()];
    let result = handle_topic(state.clone(), user_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert!(!messages.is_empty());
    
    // Verify topic was set
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.topic, Some("Operator topic".to_string()));
}

#[tokio::test]
async fn test_topic_not_in_channel() {
    let (state, user_id, _) = setup_test_state().await;
    
    // Remove user from channel
    {
        let state_guard = state.write().await;
        let mut channel = state_guard.channels.get_mut("#test").unwrap();
        channel.members.remove(&user_id);
    }
    
    let params = vec!["#test".to_string(), "Not allowed".to_string()];
    let result = handle_topic(state, user_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NotOnChannel error
    assert!(messages[0].to_string().contains("442")); // ERR_NOTONCHANNEL
}

#[tokio::test]
async fn test_topic_nonexistent_channel() {
    let (state, user_id, _) = setup_test_state().await;
    
    let params = vec!["#nonexistent".to_string(), "No channel".to_string()];
    let result = handle_topic(state, user_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NoSuchChannel error
    assert!(messages[0].to_string().contains("403")); // ERR_NOSUCHCHANNEL
}

#[tokio::test]
async fn test_topic_no_params() {
    let (state, user_id, _) = setup_test_state().await;
    
    let params = vec![]; // No parameters
    let result = handle_topic(state, user_id, params).await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "TOPIC command requires channel parameter");
}