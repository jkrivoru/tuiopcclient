---
name: Release
about: Release checklist template
title: 'Release v[VERSION]'
labels: 'release'
assignees: ''
---

## Release Checklist

### Pre-release
- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md` with new features and fixes
- [ ] Update documentation if needed
- [ ] Run full test suite locally
- [ ] Verify CI passes on main branch

### Release
- [ ] Create and push git tag: `git tag v[VERSION] && git push origin v[VERSION]`
- [ ] Verify GitHub Actions release workflow completes successfully
- [ ] Verify all platform binaries are built and uploaded
- [ ] Test downloaded binaries on different platforms

### Post-release
- [ ] Update release notes with detailed changelog
- [ ] Announce release (if applicable)
- [ ] Close milestone (if applicable)

### Notes
Add any additional notes about this release here.
