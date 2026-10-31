# Changelog

All notable production changes will be recorded here. The project follows
Semantic Versioning; entries are grouped under Added, Changed, Fixed, Security,
and Removed.

## Unreleased

## [1.0.0]

### Added

- First stable API baseline for teams, worksets, comics, chapters, assignments,
  translation units, artwork review, and termbases.
- Permanent comic archives and complete monthly JSON exports with memory, disk,
  and concurrency budgets.
- Organization-transfer CI/CD, production deployment, migration, rollback,
  retention, and post-deployment verification infrastructure.
- Versioned release notes, source archives, checksums, CycloneDX SBOM, and
  deployment evidence validation for GitHub Actions releases.

### Changed

- Unified all workspace package versions and OpenAPI metadata at `1.0.0`.
- Changed the repository and workspace package license from MIT to
  `AGPL-3.0-only`.

### Fixed

- Consistent Chapter-before-Comic lock acquisition for chapter creation,
  deletion, hierarchy marking, and comic archiving.

### Security

- Updated affected dependencies and removed the temporary ZIP advisory ignores.

### Removed

- User account deletion from the supported API; profile reads, updates, and
  avatar operations remain available.

See [the 1.0.0 release notes](docs/releases/1.0.0.md) for compatibility and
deployment requirements.
