#!/bin/bash

# Release script for opcua-client
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh v0.1.1

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.1.1"
    exit 1
fi

VERSION=$1
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

echo "🚀 Starting release process for $VERSION"

# Validate version format
if [[ ! $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "❌ Invalid version format. Use semantic versioning like v1.0.0"
    exit 1
fi

# Check if we're on main branch
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "⚠️  Warning: You're not on the main branch. Current branch: $CURRENT_BRANCH"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "❌ You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Extract version number without 'v' prefix
VERSION_NUMBER=${VERSION#v}

echo "📝 Updating Cargo.toml version to $VERSION_NUMBER"
sed -i.bak "s/^version = \".*\"/version = \"$VERSION_NUMBER\"/" Cargo.toml
rm Cargo.toml.bak

echo "📝 Updating CHANGELOG.md"
# Update the [Unreleased] section to the new version
TODAY=$(date +%Y-%m-%d)
sed -i.bak "s/## \[Unreleased\]/## [Unreleased]\n\n## [$VERSION_NUMBER] - $TODAY/" CHANGELOG.md
rm CHANGELOG.md.bak

echo "🔨 Building to verify everything works"
cargo build --release

echo "🧪 Running tests"
cargo test

echo "📋 Preparing commit"
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "chore: bump version to $VERSION"

echo "🏷️  Creating git tag"
git tag -a "$VERSION" -m "Release $VERSION"

echo "📤 Pushing changes and tag"
git push origin "$CURRENT_BRANCH"
git push origin "$VERSION"

echo "✅ Release $VERSION has been created!"
echo "🔗 GitHub Actions will automatically build and create the release."
echo "🔗 Monitor the progress at: https://github.com/yourusername/jk-opc-client/actions"
echo ""
echo "📋 Next steps:"
echo "1. Wait for the GitHub Actions release workflow to complete"
echo "2. Edit the release notes on GitHub if needed"
echo "3. Announce the release"
