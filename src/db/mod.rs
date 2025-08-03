use sqlx::{Pool, Postgres, Sqlite};
use std::sync::Arc;
use thiserror::Error;

pub mod models;
pub mod queries;
pub mod migrations;

use self::models::{User, Channel, ChannelMember, BanEntry, ServerConfig};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] sqlx::Error),
    
    #[error("User not found")]
    UserNotFound,
    
    #[error("Channel not found")]
    ChannelNotFound,
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Duplicate entry")]
    DuplicateEntry,
}

#[derive(Clone)]
pub enum Database {
    Postgres(Arc<Pool<Postgres>>),
    Sqlite(Arc<Pool<Sqlite>>),
}

impl Database {
    pub async fn connect_postgres(url: &str) -> Result<Self, DatabaseError> {
        let pool = Pool::<Postgres>::connect(url).await?;
        Ok(Database::Postgres(Arc::new(pool)))
    }
    
    pub async fn connect_sqlite(url: &str) -> Result<Self, DatabaseError> {
        let pool = Pool::<Sqlite>::connect(url).await?;
        Ok(Database::Sqlite(Arc::new(pool)))
    }
    
    pub async fn run_migrations(&self) -> Result<(), DatabaseError> {
        // TODO: Implement migrations
        Ok(())
    }
    
    // User operations
    pub async fn create_user(&self, user: &User) -> Result<User, DatabaseError> {
        queries::users::create_user(self, user).await
    }
    
    pub async fn get_user_by_nickname(&self, nickname: &str) -> Result<Option<User>, DatabaseError> {
        queries::users::get_user_by_nickname(self, nickname).await
    }
    
    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, DatabaseError> {
        queries::users::get_user_by_id(self, id).await
    }
    
    pub async fn update_user(&self, user: &User) -> Result<(), DatabaseError> {
        queries::users::update_user(self, user).await
    }
    
    pub async fn authenticate_user(&self, nickname: &str, password: &str) -> Result<User, DatabaseError> {
        queries::users::authenticate_user(self, nickname, password).await
    }
    
    // Channel operations
    pub async fn create_channel(&self, channel: &Channel) -> Result<Channel, DatabaseError> {
        queries::channels::create_channel(self, channel).await
    }
    
    pub async fn get_channel(&self, name: &str) -> Result<Option<Channel>, DatabaseError> {
        queries::channels::get_channel(self, name).await
    }
    
    pub async fn update_channel(&self, channel: &Channel) -> Result<(), DatabaseError> {
        queries::channels::update_channel(self, channel).await
    }
    
    pub async fn delete_channel(&self, name: &str) -> Result<(), DatabaseError> {
        queries::channels::delete_channel(self, name).await
    }
    
    pub async fn list_channels(&self) -> Result<Vec<Channel>, DatabaseError> {
        queries::channels::list_channels(self).await
    }
    
    // Channel membership
    pub async fn add_channel_member(&self, member: &ChannelMember) -> Result<(), DatabaseError> {
        queries::channels::add_channel_member(self, member).await
    }
    
    pub async fn remove_channel_member(&self, channel: &str, user_id: &str) -> Result<(), DatabaseError> {
        queries::channels::remove_channel_member(self, channel, user_id).await
    }
    
    pub async fn get_channel_members(&self, channel: &str) -> Result<Vec<ChannelMember>, DatabaseError> {
        queries::channels::get_channel_members(self, channel).await
    }
    
    pub async fn get_user_channels(&self, user_id: &str) -> Result<Vec<String>, DatabaseError> {
        queries::channels::get_user_channels(self, user_id).await
    }
    
    // Ban operations
    pub async fn add_ban(&self, ban: &BanEntry) -> Result<(), DatabaseError> {
        queries::bans::add_ban(self, ban).await
    }
    
    pub async fn remove_ban(&self, channel: &str, mask: &str) -> Result<(), DatabaseError> {
        queries::bans::remove_ban(self, channel, mask).await
    }
    
    pub async fn get_channel_bans(&self, channel: &str) -> Result<Vec<BanEntry>, DatabaseError> {
        queries::bans::get_channel_bans(self, channel).await
    }
    
    pub async fn is_banned(&self, channel: &str, user_mask: &str) -> Result<bool, DatabaseError> {
        queries::bans::is_banned(self, channel, user_mask).await
    }
    
    // Server configuration
    pub async fn get_server_config(&self) -> Result<ServerConfig, DatabaseError> {
        queries::config::get_server_config(self).await
    }
    
    pub async fn update_server_config(&self, config: &ServerConfig) -> Result<(), DatabaseError> {
        queries::config::update_server_config(self, config).await
    }
}