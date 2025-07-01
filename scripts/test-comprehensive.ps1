# Comprehensive Docker-based testing using Docker Compose for Windows
# This script runs multiple test scenarios

param(
    [switch]$SkipCleanup = $false
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "🐳 Comprehensive Docker Testing Suite" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

# Check if Docker and Docker Compose are available
try {
    $null = Get-Command docker -ErrorAction Stop
} catch {
    Write-Host "❌ Docker is not installed or not in PATH" -ForegroundColor Red
    exit 1
}

# Check for Docker Compose
$composeCmd = $null
try {
    $null = Get-Command docker-compose -ErrorAction Stop
    $composeCmd = "docker-compose"
} catch {
    try {
        $null = docker compose version 2>$null
        $composeCmd = "docker compose"
    } catch {
        Write-Host "❌ Docker Compose is not available" -ForegroundColor Red
        Write-Host "Please install docker-compose or use Docker with compose plugin" -ForegroundColor Yellow
        exit 1
    }
}

Set-Location $projectRoot

Write-Host "📁 Working directory: $(Get-Location)" -ForegroundColor Blue
Write-Host "🔧 Using compose command: $composeCmd" -ForegroundColor Blue
Write-Host ""

# Function to run a test service
function Invoke-TestService {
    param(
        [string]$ServiceName,
        [string]$Description
    )
    
    Write-Host "🧪 Running $Description..." -ForegroundColor Yellow
    Write-Host "   Service: $ServiceName" -ForegroundColor Gray
    
    try {
        if ($composeCmd -eq "docker-compose") {
            & docker-compose -f docker/docker-compose.test.yml build $ServiceName
            & docker-compose -f docker/docker-compose.test.yml run --rm $ServiceName
        } else {
            & docker compose -f docker/docker-compose.test.yml build $ServiceName
            & docker compose -f docker/docker-compose.test.yml run --rm $ServiceName
        }
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ $Description completed successfully" -ForegroundColor Green
            return $true
        } else {
            throw "Service failed with exit code $LASTEXITCODE"
        }
    } catch {
        Write-Host "❌ $Description failed" -ForegroundColor Red
        return $false
    }
}

# Arrays to track test results
$failedTests = @()
$successfulTests = @()

Write-Host "🏗️  Building base images..." -ForegroundColor Blue
try {
    if ($composeCmd -eq "docker-compose") {
        & docker-compose -f docker/docker-compose.test.yml build --parallel
    } else {
        & docker compose -f docker/docker-compose.test.yml build --parallel
    }
} catch {
    Write-Host "⚠️  Parallel build failed, continuing with individual builds..." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🧪 Running test suite..." -ForegroundColor Yellow
Write-Host ""

# Test 1: Linux cross-compilation
if (Invoke-TestService "test-linux" "Linux Cross-Compilation Test") {
    $successfulTests += "Linux Cross-Compilation"
} else {
    $failedTests += "Linux Cross-Compilation"
}

Write-Host ""

# Test 2: Build script verification
if (Invoke-TestService "test-verify" "Build Script Verification") {
    $successfulTests += "Build Script Verification"
} else {
    $failedTests += "Build Script Verification"
}

Write-Host ""

# Test 3: Alpine minimal build
if (Invoke-TestService "test-alpine" "Alpine Minimal Build Test") {
    $successfulTests += "Alpine Minimal Build"
} else {
    $failedTests += "Alpine Minimal Build"
}

Write-Host ""

# Test 4: Ubuntu comprehensive build
if (Invoke-TestService "test-ubuntu" "Ubuntu Comprehensive Build Test") {
    $successfulTests += "Ubuntu Comprehensive Build"
} else {
    $failedTests += "Ubuntu Comprehensive Build"
}

Write-Host ""

# Test 5: Specific Rust version
if (Invoke-TestService "test-rust-stable" "Rust Stable Version Test") {
    $successfulTests += "Rust Stable Version"
} else {
    $failedTests += "Rust Stable Version"
}

Write-Host ""

# Cleanup
if (-not $SkipCleanup) {
    Write-Host "🧹 Cleaning up..." -ForegroundColor Blue
    try {
        if ($composeCmd -eq "docker-compose") {
            & docker-compose -f docker/docker-compose.test.yml down --volumes --remove-orphans
        } else {
            & docker compose -f docker/docker-compose.test.yml down --volumes --remove-orphans
        }
    } catch {
        Write-Host "⚠️  Cleanup encountered issues, but continuing..." -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "📊 Comprehensive Test Summary:" -ForegroundColor Cyan
Write-Host "==============================" -ForegroundColor Cyan

if ($successfulTests.Count -gt 0) {
    Write-Host "✅ Successful tests ($($successfulTests.Count)):" -ForegroundColor Green
    foreach ($test in $successfulTests) {
        Write-Host "   - $test"
    }
}

if ($failedTests.Count -gt 0) {
    Write-Host "❌ Failed tests ($($failedTests.Count)):" -ForegroundColor Red
    foreach ($test in $failedTests) {
        Write-Host "   - $test"
    }
    Write-Host ""
    Write-Host "⚠️  Some tests failed. Review the output above for details." -ForegroundColor Yellow
    exit 1
} else {
    Write-Host ""
    Write-Host "🎉 All tests passed! Your cross-compilation setup is working perfectly." -ForegroundColor Green
    Write-Host "🚀 Ready for production releases!" -ForegroundColor Green
}
