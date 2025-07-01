# Docker-based build testing script for Windows
# This script uses Docker to test cross-compilation in isolated environments

param(
    [switch]$KeepLogs = $false
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "🐳 Docker Build Testing for OPC UA Client" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Check if Docker is available
try {
    $null = Get-Command docker -ErrorAction Stop
} catch {
    Write-Host "❌ Docker is not installed or not in PATH" -ForegroundColor Red
    Write-Host "Please install Docker Desktop and try again" -ForegroundColor Yellow
    exit 1
}

# Check if Docker daemon is running
try {
    $null = docker info 2>$null
} catch {
    Write-Host "❌ Docker daemon is not running" -ForegroundColor Red
    Write-Host "Please start Docker Desktop and try again" -ForegroundColor Yellow
    exit 1
}

Set-Location $projectRoot

Write-Host "📁 Working directory: $(Get-Location)" -ForegroundColor Blue
Write-Host ""

# Function to run Docker build with error handling
function Invoke-DockerTest {
    param(
        [string]$TestName,
        [string]$Dockerfile,
        [string]$Target
    )
    
    Write-Host "🔨 Running $TestName..." -ForegroundColor Blue
    
    $logFile = "docker-test-$Target.log"
    
    try {
        $output = docker build -f $Dockerfile --target $Target --progress=plain . 2>&1
        $output | Tee-Object -FilePath $logFile
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ $TestName completed successfully" -ForegroundColor Green
            return $true
        } else {
            throw "Docker build failed with exit code $LASTEXITCODE"
        }
    } catch {
        Write-Host "❌ $TestName failed" -ForegroundColor Red
        Write-Host "📋 Check $logFile for details" -ForegroundColor Yellow
        return $false
    }
}

# Arrays to track test results
$failedTests = @()
$successfulTests = @()

Write-Host "🧪 Testing Linux cross-compilation builds..." -ForegroundColor Yellow
if (Invoke-DockerTest "Linux Cross-Compilation Test" "docker/Dockerfile.test" "test-linux") {
    $successfulTests += "Linux Cross-Compilation"
} else {
    $failedTests += "Linux Cross-Compilation"
}

Write-Host ""
Write-Host "🧪 Testing build verification script..." -ForegroundColor Yellow
if (Invoke-DockerTest "Build Script Verification" "docker/Dockerfile.test" "verify") {
    $successfulTests += "Build Script Verification"
} else {
    $failedTests += "Build Script Verification"
}

Write-Host ""
Write-Host "📊 Docker Test Summary:" -ForegroundColor Cyan
Write-Host "======================" -ForegroundColor Cyan

if ($successfulTests.Count -gt 0) {
    Write-Host "✅ Successful tests:" -ForegroundColor Green
    foreach ($test in $successfulTests) {
        Write-Host "   - $test"
    }
}

if ($failedTests.Count -gt 0) {
    Write-Host "❌ Failed tests:" -ForegroundColor Red
    foreach ($test in $failedTests) {
        Write-Host "   - $test"
    }
    Write-Host ""
    Write-Host "⚠️  Some tests failed. Check the log files for details:" -ForegroundColor Yellow
    $logFiles = Get-ChildItem -Name "docker-test-*.log" -ErrorAction SilentlyContinue
    if ($logFiles) {
        foreach ($file in $logFiles) {
            Write-Host "   $file"
        }
    } else {
        Write-Host "   No log files found"
    }
    exit 1
} else {
    Write-Host ""
    Write-Host "🎉 All Docker tests passed! Cross-compilation setup is working correctly." -ForegroundColor Green
}

# Clean up log files on success (unless -KeepLogs is specified)
if (-not $KeepLogs) {
    Write-Host ""
    Write-Host "🧹 Cleaning up..." -ForegroundColor Blue
    Get-ChildItem -Name "docker-test-*.log" -ErrorAction SilentlyContinue | Remove-Item
    Write-Host "✅ Cleanup complete" -ForegroundColor Green
}
