# 🏛️ Legion Server

A modern IRC server built in Rust with IRCv3 support and Legion Protocol integration.

## Features

- **IRCv3 Protocol**: Message tags, server-time, SASL authentication, capabilities
- **Modern Architecture**: Actor-based design with async I/O and memory safety
- **Database Support**: PostgreSQL and SQLite persistence 
- **Legion Protocol**: Optimized for Legion ecosystem and Legionnaire client
- **Security**: Rate limiting, flood protection, TLS/SSL support

## Installation

```bash
cargo install legion-server
```

## Quick Start

```bash
# Start server with default config
legion-server

# Custom configuration
legion-server --config server.toml

# Listen on specific port
legion-server --port 6667
```

## Configuration

Basic configuration in `server.toml`:

```toml
[server]
name = "irc.example.com"
host = "0.0.0.0"
port = 6667
ssl_port = 6697

[ssl]
cert_file = "server.crt"
key_file = "server.key"

[database]
type = "sqlite"  # or "postgresql"
url = "irc.db"

[limits]
max_channels = 50
max_connections = 1000
message_rate = 10
```

## IRCv3 Support

- `message-tags` - Message metadata
- `server-time` - Accurate timestamps
- `batch` - Message batching
- `echo-message` - Message echoing
- `sasl` - Authentication
- Channel modes: `+ontmislpkv`
- User modes: `+ov` (operator, voice)

## License

MIT OR Apache-2.0