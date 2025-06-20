# Installation Guide

## Installing Rust

Before you can build and run this OPC UA client, you need to install Rust:

### Windows

1. **Download and install Rust:**
    - Go to https://rustup.rs/
    - Download and run `rustup-init.exe`
    - Follow the installation prompts
    - Choose the default installation options

2. **Restart your terminal** (or VS Code) after installation

3. **Verify installation:**
   ```powershell
   rustc --version
   cargo --version
   ```

### Alternative: Using Package Managers

**Using Chocolatey:**

```powershell
choco install rust
```

**Using Scoop:**

```powershell
scoop install rust
```

**Using winget:**

```powershell
winget install Rustlang.Rust.MSVC
```

## Building the Project

Once Rust is installed:

1. Open a terminal in the project directory
2. Build the project:
   ```bash
   cargo build
   ```

3. Run the project:
   ```bash
   cargo run
   ```

## Development

- Use `cargo build` for debug builds
- Use `cargo build --release` for optimized release builds
- Use `cargo run` to build and run in one command
- Use `cargo check` for fast compilation checking without producing executables

## Troubleshooting

If you encounter issues:

1. **Path issues:** Make sure Rust is in your PATH environment variable
2. **Compilation errors:** Check that all dependencies are compatible
3. **Network issues:** Some corporate networks may block cargo's package downloads
4. **Proxy settings:** Configure cargo proxy if behind a corporate firewall:
   ```bash
   # In .cargo/config.toml
   [http]
   proxy = "http://proxy.company.com:8080"
   ```
