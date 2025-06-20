# OPC UA Client

A Rust-based OPC UA client with a DOS-style terminal user interface inspired by the classic MS-DOS Editor.

## Features

- **Connect to OPC UA servers** with various security policies
- **Browse OPC UA server nodes** in a hierarchical tree structure
- **Subscribe to nodes** for real-time value monitoring
- **Read node properties** including data type, value, and metadata
- **Write values** to writable nodes
- **DOS-style UI** with menu bar, status bar, and keyboard navigation
- **Multiple security modes** support (None, Basic128Rsa15, Basic256, etc.)

## Requirements

- Rust 1.70 or later
- Windows, Linux, or macOS

## Installation

1. Clone or download this repository
2. Navigate to the project directory
3. Build the project:

```bash
cargo build --release
```

## Usage

Run the application:

```bash
cargo run
```

### Navigation

#### Global Hotkeys

- `Alt+F` - File menu / Connect to server
- `Alt+H` - Help screen
- `Alt+X` - Exit application

#### Main Screen

- `1` - Browse OPC Server
- `2` - View Subscriptions
- `3` - Help
- `C` - Connect to server
- `D` - Disconnect from server
- `Q` - Quit application

#### Browse Screen

- `↑↓` - Navigate nodes
- `Enter` - Browse into folder
- `A` - Add node to subscription
- `P` - View node properties
- `W` - Write value to node
- `B` - Go back to parent node
- `Esc` - Return to main menu

#### Subscription Screen

- `↑↓` - Navigate subscriptions
- `D` or `Del` - Remove from subscription
- `Esc` - Return to main menu

## Configuration

The client supports various OPC UA security policies:

- None (no encryption)
- Basic128Rsa15
- Basic256
- Basic256Sha256
- Aes128Sha256RsaOaep
- Aes256Sha256RsaPss

Default server URL is `opc.tcp://localhost:4840` but can be changed in the connect dialog.

## Dependencies

- `opcua` - OPC UA client library for Rust
- `ratatui` - Terminal user interface library
- `crossterm` - Cross-platform terminal manipulation
- `tokio` - Asynchronous runtime
- `serde` - Serialization framework
- `anyhow` - Error handling

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
