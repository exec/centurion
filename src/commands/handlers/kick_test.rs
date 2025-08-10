use super::*;
use crate::state::{ServerState, Connection, Channel};
use crate::protocol::{Message, Reply};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    
async fn setup_test_state() -> (Arc<RwLock<ServerState>>, u64, u64, u64) {
    let state = Arc::new(RwLock::new(ServerState::new()));
        
        // Create test connections
        let (tx1, _) = mpsc::channel(100);
        let (tx2, _) = mpsc::channel(100);
        let (tx3, _) = mpsc::channel(100);
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
        
        let kicker_id = 1;
        let target_id = 2;
        let member_id = 3;
        
        let mut state_guard = state.write().await;
        
        // Add connections
        let mut kicker = Connection::new(kicker_id, addr, tx1);
        kicker.nickname = Some("kicker".to_string());
        kicker.username = Some("kicker".to_string());
        kicker.registered = true;
        state_guard.connections.insert(kicker_id, kicker);
        state_guard.nicknames.insert("kicker".to_string(), kicker_id);
        
        let mut target = Connection::new(target_id, addr, tx2);
        target.nickname = Some("target".to_string());
        target.username = Some("target".to_string());
        target.registered = true;
        state_guard.connections.insert(target_id, target);
        state_guard.nicknames.insert("target".to_string(), target_id);
        
        let mut member = Connection::new(member_id, addr, tx3);
        member.nickname = Some("member".to_string());
        member.username = Some("member".to_string());
        member.registered = true;
        state_guard.connections.insert(member_id, member);
        state_guard.nicknames.insert("member".to_string(), member_id);
        
        // Create test channel
        let channel = Channel::new("#test".to_string());
        channel.add_member(kicker_id, true); // true = is operator
        channel.add_member(target_id, false);
        channel.add_member(member_id, false);
        
        state_guard.channels.insert("#test".to_string(), channel);
        
        drop(state_guard);
        
        (state, kicker_id, target_id, member_id)
    }
    
    #[tokio::test]
    async fn test_kick_success() {
        let (state, kicker_id, _, _) = setup_test_state().await;
        
        let params = vec!["#test".to_string(), "target".to_string(), "Get out!".to_string()];
        let result = handle_kick(state.clone(), kicker_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert!(!messages.is_empty());
        
        // Check that target was removed from channel
        let state_guard = state.read().await;
        let channel = state_guard.channels.get("#test").unwrap();
        assert!(!channel.is_member(2)); // target_id = 2
    }
    
    #[tokio::test]
    async fn test_kick_without_reason() {
        let (state, kicker_id, _, _) = setup_test_state().await;
        
        let params = vec!["#test".to_string(), "target".to_string()];
        let result = handle_kick(state.clone(), kicker_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert!(!messages.is_empty());
        
        // Verify default reason is used
        let kick_msg = &messages[0];
        assert_eq!(kick_msg.command, "KICK");
        assert_eq!(kick_msg.params[2], "Kicked");
    }
    
    #[tokio::test]
    async fn test_kick_insufficient_params() {
        let (state, kicker_id, _, _) = setup_test_state().await;
        
        let params = vec!["#test".to_string()]; // Missing target
        let result = handle_kick(state, kicker_id, params).await;
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "KICK command requires channel and nick parameters");
    }
    
    #[tokio::test]
    async fn test_kick_not_operator() {
        let (state, _, target_id, _) = setup_test_state().await;
        
        let params = vec!["#test".to_string(), "kicker".to_string()];
        let result = handle_kick(state, target_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        
        // Should receive ChanOpPrivsNeeded error
        let msg = &messages[0];
        assert!(msg.to_string().contains("482")); // ERR_CHANOPRIVSNEEDED
    }
    
    #[tokio::test]
    async fn test_kick_not_in_channel() {
        let (state, kicker_id, _, _) = setup_test_state().await;
        
        // Remove kicker from channel first
        {
            let mut state_guard = state.write().await;
            let channel = state_guard.channels.get_mut("#test").unwrap();
            channel.members.remove(&kicker_id);
        }
        
        let params = vec!["#test".to_string(), "target".to_string()];
        let result = handle_kick(state, kicker_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        
        // Should receive NotOnChannel error
        let msg = &messages[0];
        assert!(msg.to_string().contains("442")); // ERR_NOTONCHANNEL
    }
    
    #[tokio::test]
    async fn test_kick_target_not_in_channel() {
        let (state, kicker_id, _, _) = setup_test_state().await;
        
        let params = vec!["#test".to_string(), "nonexistent".to_string()];
        let result = handle_kick(state, kicker_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        
        // Should receive NoSuchNick error
        let msg = &messages[0];
        assert!(msg.to_string().contains("401")); // ERR_NOSUCHNICK
    }
    
    #[tokio::test]
    async fn test_kick_nonexistent_channel() {
        let (state, kicker_id, _, _) = setup_test_state().await;
        
        let params = vec!["#nonexistent".to_string(), "target".to_string()];
        let result = handle_kick(state, kicker_id, params).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        
        // Should receive NoSuchChannel error
        let msg = &messages[0];
        assert!(msg.to_string().contains("403")); // ERR_NOSUCHCHANNEL
    }
    
    #[tokio::test]
    async fn test_kick_removes_target_from_channel() {
        let (state, kicker_id, target_id, _member_id) = setup_test_state().await;
        
        // Create a channel with only two members
        {
            let state_guard = state.write().await;
            let channel = Channel::new("#twomembers".to_string());
            channel.add_member(kicker_id, true); // true = is operator
            channel.add_member(target_id, false);
            state_guard.channels.insert("#twomembers".to_string(), channel);
        }
        
        let params = vec!["#twomembers".to_string(), "target".to_string()];
        let result = handle_kick(state.clone(), kicker_id, params).await;
        
        assert!(result.is_ok());
        
        // Verify target was removed from channel
        let state_guard = state.read().await;
        let channel = state_guard.channels.get("#twomembers").unwrap();
        assert!(!channel.is_member(target_id));
        assert_eq!(channel.member_count(), 1); // Only kicker remains
    }