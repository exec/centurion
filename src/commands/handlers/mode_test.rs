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
    
    let op_id = 1;
    let member_id = 2;
    let non_member_id = 3;
    
    let mut state_guard = state.write().await;
    
    // Add connections
    let mut op_conn = Connection::new(op_id, addr, tx1);
    op_conn.nickname = Some("op".to_string());
    op_conn.username = Some("op".to_string());
    op_conn.hostname = "test.host".to_string();
    op_conn.registered = true;
    state_guard.connections.insert(op_id, op_conn);
    state_guard.nicknames.insert("op".to_string(), op_id);
    
    let mut member_conn = Connection::new(member_id, addr, tx2);
    member_conn.nickname = Some("member".to_string());
    member_conn.username = Some("member".to_string());
    member_conn.hostname = "member.host".to_string();
    member_conn.registered = true;
    state_guard.connections.insert(member_id, member_conn);
    state_guard.nicknames.insert("member".to_string(), member_id);
    
    let mut non_member_conn = Connection::new(non_member_id, addr, tx3);
    non_member_conn.nickname = Some("outsider".to_string());
    non_member_conn.username = Some("outsider".to_string());
    non_member_conn.hostname = "outsider.host".to_string();
    non_member_conn.registered = true;
    state_guard.connections.insert(non_member_id, non_member_conn);
    state_guard.nicknames.insert("outsider".to_string(), non_member_id);
    
    // Create test channel
    let mut channel = Channel::new("#test".to_string());
    channel.add_member(op_id, true); // op is channel operator
    channel.add_member(member_id, false); // member is regular member
    state_guard.channels.insert("#test".to_string(), channel);
    
    drop(state_guard);
    
    (state, op_id, member_id, non_member_id)
}

#[tokio::test]
async fn test_mode_query_channel() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string()];
    let result = handle_mode(state, op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get channel mode reply
    assert_eq!(messages[0].command, "324");
    assert_eq!(messages[0].params[0], "op");
    assert_eq!(messages[0].params[1], "#test");
    assert_eq!(messages[0].params[2], "+"); // No modes set initially
}

#[tokio::test]
async fn test_mode_grant_operator() {
    let (state, op_id, member_id, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+o".to_string(), "member".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    assert!(messages.len() >= 1);
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert!(mode_msg.prefix.is_some());
    assert_eq!(mode_msg.params[0], "#test");
    assert_eq!(mode_msg.params[1], "+o");
    assert_eq!(mode_msg.params[2], "member");
    
    // Verify member is now operator
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert!(channel.is_operator(member_id));
}

#[tokio::test]
async fn test_mode_remove_operator() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    // First grant op to member
    let params = vec!["#test".to_string(), "+o".to_string(), "member".to_string()];
    let _ = handle_mode(state.clone(), op_id, params).await;
    
    // Now remove op
    let params = vec!["#test".to_string(), "-o".to_string(), "member".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "-o");
    assert_eq!(mode_msg.params[2], "member");
}

#[tokio::test]
async fn test_mode_grant_voice() {
    let (state, op_id, member_id, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+v".to_string(), "member".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "+v");
    assert_eq!(mode_msg.params[2], "member");
    
    // Verify member has voice
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    let member = channel.members.get(&member_id).unwrap();
    assert!(member.modes.contains(&'v'));
}

#[tokio::test]
async fn test_mode_set_topic_protection() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+t".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "+t");
    
    // Verify channel has +t mode
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert!(channel.modes.contains(&'t'));
}

#[tokio::test]
async fn test_mode_set_multiple_modes() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+tn-s".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "+t+n"); // Combined mode changes
    
    // Verify channel modes
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert!(channel.modes.contains(&'t'));
    assert!(channel.modes.contains(&'n'));
}

#[tokio::test]
async fn test_mode_set_channel_key() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+k".to_string(), "secret".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "+k");
    assert_eq!(mode_msg.params[2], "secret");
    
    // Verify channel has key
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.key, Some("secret".to_string()));
    assert!(channel.modes.contains(&'k'));
}

#[tokio::test]
async fn test_mode_set_channel_limit() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+l".to_string(), "10".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "+l");
    assert_eq!(mode_msg.params[2], "10");
    
    // Verify channel has limit
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.limit, Some(10));
    assert!(channel.modes.contains(&'l'));
}

#[tokio::test]
async fn test_mode_non_operator_denied() {
    let (state, _, member_id, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+t".to_string()];
    let result = handle_mode(state, member_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get ChanOpPrivsNeeded error
    assert_eq!(messages[0].command, "482");
    assert_eq!(messages[0].params[0], "member");
    assert_eq!(messages[0].params[1], "#test");
}

#[tokio::test]
async fn test_mode_non_member_denied() {
    let (state, _, _, non_member_id) = setup_test_state().await;
    
    let params = vec!["#test".to_string()];
    let result = handle_mode(state, non_member_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NotOnChannel error
    assert_eq!(messages[0].command, "442");
    assert_eq!(messages[0].params[0], "outsider");
    assert_eq!(messages[0].params[1], "#test");
}

#[tokio::test]
async fn test_mode_grant_op_to_non_member() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+o".to_string(), "outsider".to_string()];
    let result = handle_mode(state, op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get UserNotInChannel error
    assert_eq!(messages[0].command, "441");
    assert_eq!(messages[0].params[0], "op");
    assert_eq!(messages[0].params[1], "outsider");
    assert_eq!(messages[0].params[2], "#test");
}

#[tokio::test]
async fn test_mode_grant_op_to_unknown_nick() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#test".to_string(), "+o".to_string(), "nobody".to_string()];
    let result = handle_mode(state, op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NoSuchNick error
    assert_eq!(messages[0].command, "401");
    assert_eq!(messages[0].params[0], "op");
    assert_eq!(messages[0].params[1], "nobody");
}

#[tokio::test]
async fn test_mode_need_more_params() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    // +o without nick parameter
    let params = vec!["#test".to_string(), "+o".to_string()];
    let result = handle_mode(state, op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NeedMoreParams error
    assert_eq!(messages[0].command, "461");
    assert_eq!(messages[0].params[0], "op");
    assert_eq!(messages[0].params[1], "MODE");
}

#[tokio::test]
async fn test_mode_unknown_channel() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["#unknown".to_string(), "+t".to_string()];
    let result = handle_mode(state, op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    
    // Should get NoSuchChannel error
    assert_eq!(messages[0].command, "403");
    assert_eq!(messages[0].params[0], "op");
    assert_eq!(messages[0].params[1], "#unknown");
}

#[tokio::test]
async fn test_mode_no_target() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec![];
    let result = handle_mode(state, op_id, params).await;
    
    // Should return error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mode_user_mode_not_implemented() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    let params = vec!["op".to_string(), "+i".to_string()];
    let result = handle_mode(state, op_id, params).await;
    
    // Should return error for user modes
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "User modes not implemented yet");
}

#[tokio::test]
async fn test_mode_remove_key() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    // First set a key
    let params = vec!["#test".to_string(), "+k".to_string(), "secret".to_string()];
    let _ = handle_mode(state.clone(), op_id, params).await;
    
    // Now remove the key
    let params = vec!["#test".to_string(), "-k".to_string(), "secret".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "-k");
    assert_eq!(mode_msg.params[2], "*"); // Key removal shows * as parameter
    
    // Verify key is removed
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.key, None);
    assert!(!channel.modes.contains(&'k'));
}

#[tokio::test]
async fn test_mode_remove_limit() {
    let (state, op_id, _, _) = setup_test_state().await;
    
    // First set a limit
    let params = vec!["#test".to_string(), "+l".to_string(), "10".to_string()];
    let _ = handle_mode(state.clone(), op_id, params).await;
    
    // Now remove the limit
    let params = vec!["#test".to_string(), "-l".to_string()];
    let result = handle_mode(state.clone(), op_id, params).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    
    // Should get MODE message broadcast
    let mode_msg = &messages[messages.len() - 1];
    assert_eq!(mode_msg.command, "MODE");
    assert_eq!(mode_msg.params[1], "-l");
    
    // Verify limit is removed
    let state_guard = state.read().await;
    let channel = state_guard.channels.get("#test").unwrap();
    assert_eq!(channel.limit, None);
    assert!(!channel.modes.contains(&'l'));
}