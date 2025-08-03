// Migration support for database schema management

pub fn create_schema_sql() -> &'static str {
    r#"
    -- Users table
    CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        nickname TEXT UNIQUE NOT NULL,
        username TEXT NOT NULL,
        realname TEXT NOT NULL,
        password_hash TEXT NOT NULL,
        email TEXT,
        account_name TEXT,
        is_operator BOOLEAN NOT NULL DEFAULT FALSE,
        is_services BOOLEAN NOT NULL DEFAULT FALSE,
        modes TEXT NOT NULL DEFAULT '',
        away_message TEXT,
        created_at TIMESTAMP NOT NULL,
        last_seen TIMESTAMP NOT NULL,
        vhost TEXT,
        metadata JSON
    );
    
    -- Channels table
    CREATE TABLE IF NOT EXISTS channels (
        name TEXT PRIMARY KEY,
        topic TEXT,
        topic_set_by TEXT,
        topic_set_at TIMESTAMP,
        modes TEXT NOT NULL DEFAULT '',
        key TEXT,
        limit INTEGER,
        created_at TIMESTAMP NOT NULL,
        founder TEXT,
        successor TEXT,
        description TEXT,
        url TEXT,
        email TEXT,
        entry_message TEXT,
        metadata JSON
    );
    
    -- Channel members table
    CREATE TABLE IF NOT EXISTS channel_members (
        channel_name TEXT NOT NULL,
        user_id TEXT NOT NULL,
        modes TEXT NOT NULL DEFAULT '',
        joined_at TIMESTAMP NOT NULL,
        last_active TIMESTAMP NOT NULL,
        PRIMARY KEY (channel_name, user_id),
        FOREIGN KEY (channel_name) REFERENCES channels(name) ON DELETE CASCADE,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    );
    
    -- Bans table
    CREATE TABLE IF NOT EXISTS bans (
        channel_name TEXT NOT NULL,
        mask TEXT NOT NULL,
        set_by TEXT NOT NULL,
        set_at TIMESTAMP NOT NULL,
        reason TEXT,
        expires_at TIMESTAMP,
        PRIMARY KEY (channel_name, mask),
        FOREIGN KEY (channel_name) REFERENCES channels(name) ON DELETE CASCADE
    );
    
    -- Server configuration
    CREATE TABLE IF NOT EXISTS server_config (
        id INTEGER PRIMARY KEY DEFAULT 1,
        server_name TEXT NOT NULL,
        network_name TEXT NOT NULL,
        server_description TEXT NOT NULL,
        admin_name TEXT NOT NULL,
        admin_email TEXT NOT NULL,
        motd TEXT,
        max_clients INTEGER NOT NULL DEFAULT 10000,
        max_channels_per_user INTEGER NOT NULL DEFAULT 50,
        max_nickname_length INTEGER NOT NULL DEFAULT 30,
        max_channel_name_length INTEGER NOT NULL DEFAULT 50,
        max_topic_length INTEGER NOT NULL DEFAULT 390,
        max_kick_reason_length INTEGER NOT NULL DEFAULT 255,
        max_away_length INTEGER NOT NULL DEFAULT 255,
        max_message_length INTEGER NOT NULL DEFAULT 512,
        default_modes TEXT NOT NULL DEFAULT '',
        default_channel_modes TEXT NOT NULL DEFAULT '',
        ping_frequency INTEGER NOT NULL DEFAULT 120,
        ping_timeout INTEGER NOT NULL DEFAULT 60,
        flood_messages INTEGER NOT NULL DEFAULT 10,
        flood_interval INTEGER NOT NULL DEFAULT 1,
        throttle_duration INTEGER NOT NULL DEFAULT 60,
        metadata JSON
    );
    
    -- Message logs (optional, for history)
    CREATE TABLE IF NOT EXISTS message_logs (
        id TEXT PRIMARY KEY,
        timestamp TIMESTAMP NOT NULL,
        sender_id TEXT NOT NULL,
        target TEXT NOT NULL,
        message_type TEXT NOT NULL,
        content TEXT NOT NULL,
        tags JSON,
        FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE
    );
    
    -- Operator credentials
    CREATE TABLE IF NOT EXISTS operator_credentials (
        id TEXT PRIMARY KEY,
        name TEXT UNIQUE NOT NULL,
        password_hash TEXT NOT NULL,
        host_mask TEXT,
        privileges TEXT NOT NULL,
        created_at TIMESTAMP NOT NULL,
        last_used TIMESTAMP
    );
    
    -- Indexes for performance
    CREATE INDEX IF NOT EXISTS idx_users_nickname ON users(LOWER(nickname));
    CREATE INDEX IF NOT EXISTS idx_users_account ON users(account_name);
    CREATE INDEX IF NOT EXISTS idx_channel_members_user ON channel_members(user_id);
    CREATE INDEX IF NOT EXISTS idx_message_logs_timestamp ON message_logs(timestamp);
    CREATE INDEX IF NOT EXISTS idx_message_logs_target ON message_logs(target);
    "#
}