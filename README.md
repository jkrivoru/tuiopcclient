# OPC UA TUI Client

A modern, terminal-based OPC UA client built with Rust that provides an intuitive text user interface (TUI) for browsing and interacting with OPC UA servers.

![image](https://github.com/user-attachments/assets/1269ff32-2b49-4c93-a132-bd54e75be276)

## Features

### 🖥️ Terminal User Interface
- **Intuitive TUI**: Modern terminal interface built with [ratatui](https://ratatui.rs/)
- **Tree Navigation**: Browse OPC UA server node hierarchy with expandable tree view
- **Real-time Attribute Display**: View node attributes, values, and properties in real-time
- **Mouse Support**: Full mouse interaction support for navigation and selection

### 🔐 Security & Authentication
- **Multiple Security Policies**: Support for None, Basic128Rsa15, Basic256, Basic256Sha256, Aes128Sha256RsaOaep, Aes256Sha256RsaPss
- **Security Modes**: None, Sign, SignAndEncrypt
- **Certificate Management**: Client certificates, trusted certificate stores, and PKI infrastructure
- **Authentication Methods**:
  - Anonymous authentication
  - Username/password authentication
  - X.509 certificate authentication
- **Auto-trust**: Option to automatically trust server certificates

### 🔍 Advanced Search & Navigation
- **Tree Search**: Search through OPC UA node hierarchy by name, node ID, or values
- **Recursive Search**: Deep search across the entire server namespace
- **Continue Search (F3)**: Find next search results like in Windows Explorer
- **Navigation Shortcuts**: 
  - Arrow keys for tree navigation
  - Page Up/Down for faster scrolling
  - Left/Right for expand/collapse
  - Enter to expand nodes

### 🌐 Connection Management
- **Server Discovery**: Built-in OPC UA server discovery
- **Endpoint Selection**: Choose from available server endpoints with different security configurations
- **Connection Validation**: Real-time connection status monitoring
- **URL Override**: Option to use original URL instead of server-provided endpoints

### 📊 Node Browsing & Analysis
- **Complete Node Information**: Display all OPC UA node attributes
- **Node Type Recognition**: Visual indicators for Objects, Variables, Methods, Views, Types
- **Hierarchical Display**: Proper tree structure showing parent-child relationships
- **Attribute Details**: View data types, access levels, value ranks, and more

### 🛠️ Command Line Interface
- **Direct Connection**: Connect to servers directly via command line arguments
- **Batch Operations**: Automate connections with configuration files
- **Logging**: Comprehensive logging with configurable levels

## Installation

### Download Pre-built Binaries

[![Release](https://img.shields.io/github/v/release/jkrivoru/tuiopcclient)](https://github.com/jkrivoru/tuiopcclient/releases)
[![CI](https://github.com/jkrivoru/tuiopcclient/workflows/CI/badge.svg)](https://github.com/jkrivoru/tuiopcclient/actions)

Download the latest release for your platform from the [Releases page](https://github.com/jkrivoru/tuiopcclient/releases).

#### Available Platforms:
- **Linux**:
  - `opcua-client-linux-x86_64.tar.gz` - Standard Linux x86_64
  - `opcua-client-linux-x86_64-musl.tar.gz` - Static binary (musl)
  - `opcua-client-linux-aarch64.tar.gz` - ARM64/AArch64
- **Windows**:
  - `opcua-client-windows-x86_64.zip` - Windows 64-bit
  - `opcua-client-windows-i686.zip` - Windows 32-bit
- **macOS**:
  - `opcua-client-macos-x86_64.tar.gz` - Intel Mac
  - `opcua-client-macos-aarch64.tar.gz` - Apple Silicon Mac

#### Installation Steps:
1. Download the appropriate archive for your platform
2. Extract the archive: `tar -xzf opcua-client-*.tar.gz` (Linux/macOS) or use your preferred archive tool (Windows)
3. Make the binary executable (Linux/macOS): `chmod +x opcua-client`
4. Optionally, move to a directory in your PATH: `sudo mv opcua-client /usr/local/bin/`

### Building from Source

#### Prerequisites
- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **OpenSSL**: Required for secure connections (automatically handled with vendored feature)

#### Build Commands
```bash
git clone <repository-url>
cd jk-opc-client
cargo build --release
```

#### Quick Start Scripts
- **Windows**: Run `run.bat` or `run.ps1`
- **Cross-platform**: Use `cargo run --release`

## Usage

### Interactive Mode (TUI)
Launch the application to use the interactive terminal interface:

```bash
# Using pre-built binary
./opcua-client

# Or building from source
cargo run --release
```

### Command Line Mode
Connect directly to an OPC UA server:

```bash
# Basic connection (using pre-built binary)
./opcua-client --server-url "opc.tcp://localhost:4840"

# Basic connection (building from source)
cargo run --release -- --server-url "opc.tcp://localhost:4840"

# Secure connection with certificates
./opcua-client \
  --server-url "opc.tcp://localhost:4840" \
  --security-policy "Basic256Sha256" \
  --security-mode "SignAndEncrypt" \
  --client-certificate "./pki/own/cert.der" \
  --client-private-key "./pki/private/private.pem" \
  --trusted-store "./pki/trusted"

# Username/password authentication
./opcua-client \
  --server-url "opc.tcp://localhost:4840" \
  --user-name "admin" \
  --password "password"

# X.509 certificate authentication
./opcua-client \
  --server-url "opc.tcp://localhost:4840" \
  --user-certificate "./pki/user/user_cert.pem" \
  --user-private-key "./pki/user/user_key.pem"
```

### Command Line Options

| Option | Description |
|--------|-------------|
| `--server-url` | OPC UA server URL (e.g., opc.tcp://localhost:4840) |
| `--security-policy` | Security policy (None, Basic128Rsa15, Basic256, Basic256Sha256, Aes128Sha256RsaOaep, Aes256Sha256RsaPss) |
| `--security-mode` | Security mode (None, Sign, SignAndEncrypt) |
| `--client-certificate` | Path to client certificate file |
| `--client-private-key` | Path to client private key file |
| `--auto-trust` | Auto-trust server certificate |
| `--trusted-store` | Path to trusted certificate store |
| `--user-name` | Username for authentication |
| `--password` | Password for authentication |
| `--user-certificate` | Path to user certificate file for X.509 authentication |
| `--user-private-key` | Path to user private key file for X.509 authentication |
| `--use-original-url` | Use original URL instead of server-provided endpoint URLs |

## Configuration

### Configuration File
The application uses `config.json` for default settings:

```json
{
  "server_url": "opc.tcp://localhost:4840",
  "security_policy": "None",
  "username": null,
  "password": null,
  "application_name": "OPC UA Rust Client",
  "application_uri": "urn:OPC-UA-Rust-Client",
  "session_timeout": 60000,
  "keep_alive_interval": 1000
}
```

### PKI Structure
The application maintains a PKI (Public Key Infrastructure) directory structure:

```
pki/
├── own/           # Client certificates
├── private/       # Private keys
├── trusted/       # Trusted server certificates
├── rejected/      # Rejected certificates
└── user/          # User certificates for authentication
```

## Keyboard Shortcuts

### Navigation
- **Arrow Keys**: Navigate through the tree
- **Enter/Right Arrow**: Expand node
- **Left Arrow**: Collapse node or move to parent
- **Page Up/Down**: Fast scrolling
- **Home/End**: Jump to first/last node

### Search
- **Ctrl+F**: Open search dialog
- **F3**: Continue search (find next)
- **Escape**: Close search dialog
- **Tab**: Switch between search input and options

### General
- **F1**: Toggle log viewer
- **Ctrl+C**: Cancel current operation
- **Escape**: Close dialogs or exit application

## Architecture

### Core Components
- **TUI Interface**: Built with ratatui for cross-platform terminal UI
- **OPC UA Client**: Based on the [opcua](https://crates.io/crates/opcua) crate
- **Async Runtime**: Powered by [tokio](https://tokio.rs/) for concurrent operations
- **Security**: OpenSSL integration for cryptographic operations

### Module Structure
- `src/client.rs` - OPC UA client management and operations
- `src/screens/` - UI screens (connect, browse)
- `src/connection_manager.rs` - Connection handling and configuration
- `src/ui.rs` - Main application UI controller
- `src/components/` - Reusable UI components

## Development

### Building
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check code
cargo check
```

### Dependencies
Key dependencies include:
- `opcua` - OPC UA client library
- `ratatui` - Terminal UI framework
- `tokio` - Async runtime
- `crossterm` - Cross-platform terminal manipulation
- `clap` - Command line argument parsing
- `serde` - Serialization framework
- `anyhow` - Error handling

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - Verify the server URL is correct
   - Check if the OPC UA server is running
   - Ensure firewall settings allow the connection

2. **Certificate Errors**
   - Check certificate paths in command line arguments
   - Verify certificates are in the correct format (DER/PEM)
   - Use `--auto-trust` for testing with self-signed certificates

3. **Authentication Failures**
   - Verify username/password credentials
   - Check if the server supports the chosen authentication method
   - Ensure user certificates are properly configured

### Logging
The application provides detailed logging. Use the built-in log viewer (F1) or check console output for debugging information.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License

Do whatever you want. Take responsibility for your actions.  

## Acknowledgments

- Built with the excellent [opcua](https://github.com/locka99/opcua) Rust crate
- UI powered by [ratatui](https://ratatui.rs/)
- Inspired by modern terminal applications and OPC UA standards
