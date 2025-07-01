# OPC UA Client - Docker Testing Commands
# 
# Usage:
#   .\scripts\test.ps1 <command>
#
# Commands:
#   basic       - Run basic Docker build test
#   comprehensive - Run comprehensive test suite
#   alpine      - Test Alpine-based minimal build
#   ubuntu      - Test Ubuntu-based comprehensive build
#   cleanup     - Clean up Docker images and containers
#   help        - Show this help

param(
    [Parameter(Position=0)]
    [string]$Command = "help"
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

function Show-Help {
    Write-Host "OPC UA Client - Docker Testing Commands" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\scripts\test.ps1 <command>" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Commands:" -ForegroundColor Green
    Write-Host "  basic         - Run basic Docker build test" -ForegroundColor White
    Write-Host "  simple        - Run simple cross-compilation test (no cross tool)" -ForegroundColor White
    Write-Host "  comprehensive - Run comprehensive test suite with all scenarios" -ForegroundColor White
    Write-Host "  alpine        - Test Alpine-based minimal build (static binary)" -ForegroundColor White
    Write-Host "  ubuntu        - Test Ubuntu-based comprehensive build" -ForegroundColor White
    Write-Host "  local         - Run local build tests (no Docker)" -ForegroundColor White
    Write-Host "  cleanup       - Clean up Docker images and containers" -ForegroundColor White
    Write-Host "  help          - Show this help" -ForegroundColor White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\scripts\test.ps1 basic" -ForegroundColor Gray
    Write-Host "  .\scripts\test.ps1 comprehensive" -ForegroundColor Gray
    Write-Host "  .\scripts\test.ps1 cleanup" -ForegroundColor Gray
}

function Invoke-SimpleTest {
    Write-Host "Running simple cross-compilation test..." -ForegroundColor Blue
    Set-Location $projectRoot
    docker build -f docker/Dockerfile.simple --target test-simple .
}

function Invoke-BasicTest {
    Write-Host "Running basic Docker build test..." -ForegroundColor Blue
    & "$scriptDir\test-docker.ps1"
}

function Invoke-ComprehensiveTest {
    Write-Host "Running comprehensive test suite..." -ForegroundColor Blue
    & "$scriptDir\test-comprehensive.ps1"
}

function Invoke-AlpineTest {
    Write-Host "Running Alpine minimal build test..." -ForegroundColor Blue
    Set-Location $projectRoot
    docker build -f docker/Dockerfile.alpine .
}

function Invoke-UbuntuTest {
    Write-Host "Running Ubuntu comprehensive build test..." -ForegroundColor Blue
    Set-Location $projectRoot
    docker build -f docker/Dockerfile.ubuntu .
}

function Invoke-LocalTest {
    Write-Host "Running local build tests..." -ForegroundColor Blue
    if (Test-Path "$scriptDir\test-builds.ps1") {
        & "$scriptDir\test-builds.ps1"
    } else {
        Write-Host "Local test script not found, running cargo build..." -ForegroundColor Yellow
        Set-Location $projectRoot
        cargo build --release
        cargo test
    }
}

function Invoke-Cleanup {
    Write-Host "Cleaning up Docker resources..." -ForegroundColor Blue
    Set-Location $projectRoot
    
    Write-Host "Stopping containers..." -ForegroundColor Gray
    docker ps -q --filter "label=com.docker.compose.project=jk-opc-client" | ForEach-Object {
        if ($_) { docker stop $_ }
    }
    
    Write-Host "Removing containers..." -ForegroundColor Gray
    docker ps -aq --filter "label=com.docker.compose.project=jk-opc-client" | ForEach-Object {
        if ($_) { docker rm $_ }
    }
    
    Write-Host "Removing images..." -ForegroundColor Gray
    docker images --filter "reference=jk-opc-client*" -q | ForEach-Object {
        if ($_) { docker rmi $_ }
    }
    
    Write-Host "Cleaning up build cache..." -ForegroundColor Gray
    docker volume ls -q --filter "name=jk-opc-client" | ForEach-Object {
        if ($_) { docker volume rm $_ }
    }
    
    Write-Host "Cleanup complete" -ForegroundColor Green
}

# Main command dispatcher
switch ($Command.ToLower()) {
    "basic" { Invoke-BasicTest }
    "simple" { Invoke-SimpleTest }
    "comprehensive" { Invoke-ComprehensiveTest }
    "alpine" { Invoke-AlpineTest }
    "ubuntu" { Invoke-UbuntuTest }
    "local" { Invoke-LocalTest }
    "cleanup" { Invoke-Cleanup }
    "help" { Show-Help }
    default {
        Write-Host "Unknown command: $Command" -ForegroundColor Red
        Write-Host ""
        Show-Help
        exit 1
    }
}
