#!/usr/bin/env pwsh

Write-Host "Building OPC UA Client..." -ForegroundColor Green
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!" -ForegroundColor Green
    Write-Host "Running OPC UA Client..." -ForegroundColor Yellow
    cargo run --release
} else {
    Write-Host "Build failed!" -ForegroundColor Red
    Read-Host "Press Enter to continue..."
}
