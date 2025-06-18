@echo off
echo Building OPC UA Client...
cargo build --release
if %ERRORLEVEL% EQU 0 (
    echo Build successful!
    echo Running OPC UA Client...
    cargo run --release
) else (
    echo Build failed!
    pause
)
