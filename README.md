# Kafka Eye

A terminal-based Kafka client with vim-like interface built in Rust.

## Features

- **Vim-like Interface**: Navigate and operate using familiar vim keybindings
- **Real-time Monitoring**: Live view of topics, messages, and consumer groups
- **Message Production/Consumption**: Send and receive messages with ease
- **Multiple Screen Views**: Dashboard, topics, producer, consumer, groups, monitoring
- **Configuration Management**: Support for various Kafka security configurations
- **Cross-platform**: Works on Windows, Linux, and macOS

## Quick Start

### Prerequisites

- Rust 1.70+ 
- A running Kafka cluster

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/kafka-eye.git
cd kafka-eye

# Build the project
cargo build --release

# Run the application
cargo run
```

### Configuration

Create a `config.yaml` file:

```yaml
kafka:
  brokers:
    - "localhost:9092"
  client_id: "kafka-eye"
  security: ~
  producer:
    acks: "all"
    compression_type: "none"
    batch_size: 16384
    linger_ms: 0
  consumer:
    auto_offset_reset: "earliest"
    enable_auto_commit: true
    auto_commit_interval_ms: 5000
    session_timeout_ms: 30000
    heartbeat_interval_ms: 3000

ui:
  theme: "default"
  refresh_interval_ms: 1000
  max_messages: 1000
  vim_mode: true

logging:
  level: "info"
  file: ~
```

## Usage

### Vim-like Navigation

- `h/j/k/l` or arrow keys: Navigate
- `gg`: Go to top
- `G`: Go to bottom
- `Tab`/`Shift+Tab`: Switch between screens
- `q`: Quit application
- `r`: Refresh current screen

### Command Mode

Press `:` to enter command mode:

- `:topics` - Go to topics list
- `:produce <topic>` - Start producing to topic
- `:consume <topic>` - Start consuming from topic
- `:groups` - Show consumer groups
- `:connect <broker>` - Connect to specific broker
- `:quit` or `:q` - Exit application

### Insert Mode

Press `i` to enter insert mode when in producer screen:

- Type your message
- Press `Enter` to send
- Press `ESC` to return to normal mode

## Screens

1. **Dashboard**: Overview of cluster status and recent activity
2. **Topics**: List and manage Kafka topics
3. **Producer**: Send messages to topics
4. **Consumer**: Consume messages from topics
5. **Groups**: Monitor consumer groups
6. **Monitor**: Real-time metrics and statistics
7. **Settings**: Application configuration

## Development

### Project Structure

```
src/
├── main.rs           # Application entry point
├── app/              # Application logic
│   ├── mod.rs
│   ├── events.rs     # Event handling
│   └── state.rs      # Application state
├── ui/               # User interface
│   ├── mod.rs
│   ├── components/   # Reusable UI components
│   └── screens/      # Different screen views
├── kafka/            # Kafka client operations
│   ├── mod.rs
│   ├── client.rs     # Main Kafka client
│   ├── admin.rs      # Admin operations
│   ├── producer.rs   # Message production
│   └── consumer.rs   # Message consumption
├── config/           # Configuration management
│   └── mod.rs
└── utils/            # Utility functions
    ├── mod.rs
    └── error.rs      # Error handling
```

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run with debug logging
cargo run -- --debug

# Run with custom config
cargo run -- --config custom-config.yaml
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for terminal UI
- Uses [rdkafka](https://github.com/fede1024/rust-rdkafka) for Kafka operations
- Inspired by vim's modal interface design
