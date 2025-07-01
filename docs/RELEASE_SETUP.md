# Release Setup Documentation

This document describes the GitHub Actions CI/CD pipeline and release automation setup for the OPC UA TUI Client.

## Overview

The project now includes a comprehensive CI/CD pipeline that automatically:
- Tests code on multiple platforms
- Builds release binaries for common platforms
- Creates GitHub releases with downloadable assets
- Provides nightly builds for development snapshots

## GitHub Actions Workflows

### 1. CI Workflow (`.github/workflows/ci.yml`)
- **Triggers**: Push to `main`/`develop` branches, pull requests to `main`
- **Purpose**: Continuous integration testing
- **Actions**:
  - Code formatting check (`cargo fmt`)
  - Linting with Clippy (`cargo clippy`)
  - Unit tests (`cargo test`)
  - Build verification on Linux, Windows, and macOS

### 2. Release Workflow (`.github/workflows/release.yml`)
- **Triggers**: Git tags matching `v*` pattern (e.g., `v0.1.0`)
- **Purpose**: Automated release creation and binary distribution
- **Targets**:
  - **Linux**: x86_64 (glibc), x86_64 (musl), aarch64
  - **Windows**: x86_64, i686 (32-bit)
  - **macOS**: x86_64 (Intel), aarch64 (Apple Silicon)
- **Output**: GitHub release with platform-specific archives

### 3. Nightly Workflow (`.github/workflows/nightly.yml`)
- **Triggers**: Daily at 2 AM UTC, manual dispatch
- **Purpose**: Development snapshots
- **Actions**: Builds binaries for major platforms if changes detected in last 24 hours

## Cross-Compilation Setup

### Cross.toml Configuration
The `Cross.toml` file configures cross-compilation for non-native targets:
- Linux musl targets with proper toolchain setup
- ARM64 cross-compilation configuration
- Windows cross-compilation handling

### Dependencies
- **OpenSSL**: Uses vendored feature for static linking
- **Cross**: Tool for cross-compilation to different targets
- **Platform-specific toolchains**: Automatically installed in CI

## Release Process

### Automated Release (Recommended)
1. Update `Cargo.toml` version
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -m "chore: bump version to v0.1.0"`
4. Create and push tag: `git tag v0.1.0 && git push origin v0.1.0`
5. GitHub Actions automatically creates release with binaries

### Using Release Scripts
**Linux/macOS:**
```bash
chmod +x scripts/release.sh
./scripts/release.sh v0.1.0
```

**Windows:**
```powershell
.\scripts\release.ps1 v0.1.0
```

### Manual Release Steps
1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Test builds: `./scripts/test-builds.sh` (Linux/macOS)
4. Commit and tag
5. Push tag to trigger release workflow

## Platform-Specific Notes

### Linux
- **Standard (glibc)**: Works on most modern Linux distributions
- **musl**: Static binary, works on any Linux distribution
- **aarch64**: For ARM64 systems (Raspberry Pi 4, AWS Graviton, etc.)

### Windows
- **x86_64**: For 64-bit Windows systems
- **i686**: For 32-bit Windows systems (legacy support)

### macOS
- **x86_64**: For Intel-based Macs
- **aarch64**: For Apple Silicon Macs (M1, M2, etc.)

## Binary Distribution

### Download Locations
- **Releases**: https://github.com/yourusername/jk-opc-client/releases
- **Latest**: https://github.com/yourusername/jk-opc-client/releases/latest
- **Nightly**: Available as GitHub Actions artifacts (7-day retention)

### File Naming Convention
- Linux: `opcua-client-linux-{arch}.tar.gz`
- Windows: `opcua-client-windows-{arch}.zip`
- macOS: `opcua-client-macos-{arch}.tar.gz`
- Nightly: `opcua-client-nightly-{platform}-{arch}`

### Installation
1. Download appropriate archive for your platform
2. Extract: `tar -xzf file.tar.gz` (Unix) or use archive tool (Windows)
3. Make executable: `chmod +x opcua-client` (Unix)
4. Optionally move to PATH: `sudo mv opcua-client /usr/local/bin/`

## Quality Assurance

### Pre-Release Testing
- All targets must build successfully
- Tests must pass on all platforms
- Code must pass formatting and linting checks

### Security Considerations
- All builds use vendored OpenSSL for security consistency
- Reproducible builds through locked dependencies
- Static analysis with Clippy

## Troubleshooting

### Common Issues
1. **Cross-compilation failures**: Check `Cross.toml` configuration
2. **OpenSSL errors**: Verify vendored feature is enabled
3. **Tag not triggering release**: Ensure tag follows `v*` pattern
4. **Missing binaries**: Check GitHub Actions logs for build failures

## Local Testing

### Docker-based Testing (Recommended)
Use Docker for consistent, isolated testing environments:

**Quick Tests:**
```bash
# Windows
.\scripts\test.ps1 basic

# Linux/macOS  
./scripts/test.sh basic
```

**Comprehensive Testing:**
```bash
# Windows
.\scripts\test.ps1 comprehensive

# Linux/macOS
./scripts/test.sh comprehensive
```

**Specific Platform Tests:**
```bash
# Test Alpine (static binary)
.\scripts\test.ps1 alpine

# Test Ubuntu (full toolchain)  
.\scripts\test.ps1 ubuntu
```

### Local Build Testing
Use `scripts/test-builds.sh` to verify cross-compilation works locally before creating releases.

**Requirements:**
- Docker and Docker Compose
- Or: Rust toolchain with cross-compilation targets

**Available Commands:**
- `basic` - Basic Docker build verification
- `comprehensive` - Full test suite with multiple scenarios
- `alpine` - Minimal static binary testing
- `ubuntu` - Comprehensive toolchain testing
- `local` - Local build testing without Docker
- `cleanup` - Clean up Docker resources

## GitHub Repository Setup

### Required Settings
1. **Actions**: Enable GitHub Actions in repository settings
2. **Releases**: Ensure repository allows release creation
3. **Secrets**: No additional secrets required (uses `GITHUB_TOKEN`)

### Branch Protection (Recommended)
- Protect `main` branch
- Require PR reviews
- Require status checks (CI workflow)
- Require branches to be up to date

### Repository Configuration
Update the GitHub repository URLs in:
- `README.md` badges and links
- `.github/workflows/*.yml` (if using different repository name)
- Release scripts

## Next Steps

1. **Update Repository URLs**: Replace `yourusername/jk-opc-client` with actual repository
2. **Test Release Process**: Create a test tag to verify workflow
3. **Documentation**: Update any additional documentation with download links
4. **Announce**: Set up communication channels for release announcements

## Support

For issues with the release process:
1. Check GitHub Actions logs
2. Review this documentation
3. Create an issue using the provided templates
4. Verify all prerequisites are met
