# Organization Transfer Readiness Checklist

This checklist tracks the work required before moving `poprako-server` from
the personal repository into the official PopRaKo organization. Complete the
blocking items before the transfer, then verify the organization-side settings
immediately after it.

## 1. Repository identity and public entry points

- [ ] Decide the final organization and repository name.
  - Expected URL: `https://github.com/poprako-dev/poprako-server`.
  - Confirm that the target organization does not already contain a repository
    or fork with the same name.
- [x] Add a root `README.md`.
  - Describe the product and current project status.
  - Document prerequisites, local configuration, database setup, migrations,
    development commands, tests, and production build commands.
  - Link to the API specification and contribution guide.
- [x] Add the full MIT license text as root `LICENSE`.
  - Keep `license = "MIT"` in Cargo package metadata.
- [ ] Add a repository description, website, and topics on GitHub.
- [ ] Decide whether `dev` remains the default branch.
  - Prefer a protected `main` branch for a small trunk-based team.
  - If retaining `dev`, document the roles of `main`, `dev`, and release
    branches in `CONTRIBUTING.md`.

### Acceptance

- [x] A new contributor can understand and start the project from the README.
- [ ] GitHub recognizes and displays the repository license.
- [x] All Cargo package repository URLs use the final organization URL.

## 2. Community and governance files

- [ ] Add `CONTRIBUTING.md`.
  - Explain the branch and pull-request workflow.
  - List the required Rust, custom-format, and integration-test commands.
  - Distinguish standard Rust practices from PopRaKo-specific style rules.
  - Explain when `tests/integration-tests/TESTCASES.md` must be updated.
- [ ] Add `SECURITY.md` with a private vulnerability-reporting channel.
- [ ] Add or adopt an organization-level `CODE_OF_CONDUCT.md`.
- [ ] Add issue forms and a pull-request template, either locally or through
  the organization's public `.github` repository.
- [ ] Add `CODEOWNERS` after organization teams have been created.
  - Require explicit ownership for `.github/workflows/`, `migrations/`,
    `scripts/`, `Dockerfile`, and deployment configuration.
- [ ] Decide whether support and governance policies belong in this repository
  or the organization's shared `.github` repository.

### Acceptance

- [ ] Opening an issue or pull request presents the expected templates and
  contribution guidance.
- [ ] Security reports have a documented non-public path.
- [ ] Sensitive infrastructure changes request the correct team reviewers.

## 3. Clean stale and personal repository state

- [ ] Remove or generalize `.agents/settings.local.json`.
  - It currently contains personal absolute paths and an obsolete
    `poprako-r` path.
  - Shared Agent configuration must not depend on one maintainer's home
    directory.
- [ ] Refresh `src/AGENTS.md`.
  - Remove references to deleted `.bak` files, `forward_ref.rs`, and `domain/`.
  - Replace the generated layout with the authoritative active module graph.
- [ ] Refresh `docs/AGENTS.md`.
  - Remove the reference to the missing `how-to-implement-api-http.md`.
  - Reconcile the instruction to remove stale plans with the files currently
    kept under `docs/plans/`.
- [ ] Review all `NOTE.md`, `CHECK-LIST.md`, `docs/todos.md`, and
  `docs/plans/` files.
  - Keep only active references with a clear owner.
  - Delete completed migration notes and obsolete design plans.
- [ ] Remove the tracked zero-byte root `openapi.json`.
- [ ] Remove `docs/swagger.previous.json` unless a documented workflow needs
  it. It currently duplicates `docs/swagger.json` exactly.
- [ ] Decide whether `application_config.json` should remain at the root or be
  renamed as an explicit local-development example.
- [ ] Remove or clearly mark legacy `poprako-sr` names.
  - Review `deploy/poprako-sr/`.
  - Review defaults in `scripts/docker-run-prod.sh`,
    `scripts/local-run-release.sh`, and `scripts/local-stop-release.sh`.
  - Review the legacy `/opt/poprako-s/shared/.env` production dependency.

### Acceptance

- [ ] `rg 'poprako-r|poprako-sr|/Users/|/home/'` returns only documented,
  intentional compatibility references.
- [ ] Every path named by an `AGENTS.md` file exists or is clearly presented as
  an example.
- [ ] There is exactly one checked-in generated OpenAPI document.

## 4. Cargo workspace metadata and toolchain

- [x] Correct the repository URL in `poprako-util/Cargo.toml`.
  - Remove the unrelated `https://github.com/anomalyco/poprako-server` value.
- [ ] Add a workspace-level package policy in the root `Cargo.toml`.
  - Inherit the version, edition, Rust version, license, repository, and
    publishing policy where appropriate.
- [ ] Centralize shared dependency versions with `[workspace.dependencies]`.
- [ ] Centralize shared lint policy with `[workspace.lints]` and inherit it in
  every member crate.
- [x] Select and document Rust 1.95 as the supported version.
  - Add `rust-version` to workspace package metadata.
  - Add a pinned `rust-toolchain.toml`.
  - Use the same exact Rust version in the Docker builder image.
- [ ] Review Cargo features.
  - Confirm that `rdb`, `repo_impl`, and `prom_impl` are still the intended
    names and combinations.
  - Document test-only feature combinations used by RDB tests.
- [ ] Decide whether root Cargo commands should default to the server package
  or the entire workspace.
  - CI commands must always specify `--workspace` explicitly.

### Acceptance

- [x] `cargo metadata --no-deps --format-version 1` reports the correct
  repository and Rust version for all members.
- [x] Local, CI, and Docker builds use the same Rust 1.95 toolchain policy.
- [ ] No workspace member silently misses the shared lint policy.

## 5. Make repository rules enforceable

- [x] Resolve the contradictory formatting instruction in root `AGENTS.md`.
  - Read-only verification should use `cargo fmt --all --check`.
  - Code-changing workflows may run `cargo fmt --all` before verification.
- [ ] Decide whether the 600-line Rust file limit is mandatory.
  - If mandatory, add an automated check.
  - Split or explicitly waive the 15 files currently at or above 600 lines.
  - The current largest file is `src/complex/chapter_port/import.rs` at 770
    lines.
- [ ] Reconsider blanket style bans that differ from normal Rust practice.
  - In particular, decide whether `if ... else` is truly forbidden or merely
    discouraged when guards or `match` are clearer.
  - Keep project-specific rules in `CONTRIBUTING.md` and automate every hard
    requirement.
- [x] Add a portable `scripts/ci-check.sh` as the validation entry point.
  - Use `cargo fmt --all --check` instead of the mutating formatter.
  - Ensure a custom checker failure cannot be masked by a later command.
  - Check the entire workspace and all targets.
  - Treat Clippy warnings as errors.
  - Keep `just fmt-check` only as an optional local wrapper around the script.
- [x] Fix the current strict Clippy failures.
  - Remove four unused `super::*` imports in RDB test modules.
  - Resolve the unused `find_chapter` helper.
  - Replace the needless `.last()` traversal in the chapter-port import test.

### Acceptance

The following commands all succeed from a clean checkout:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
sh fmt/run-check.sh
```

## 6. Continuous integration

- [x] Add a pull-request CI workflow under `.github/workflows/`.
  - Run CI for pull requests targeting `dev` or `main`.
  - Run branch CI after changes land on `main`; do not rerun it for pushes to
    `dev` after a pull request has already passed.
- [x] Invoke `sh scripts/ci-check.sh`; do not install or require `just` in CI.
- [x] Run `cargo test --workspace` through `sh scripts/ci-test.sh`.
  - Separate fast unit tests from Docker/PostgreSQL-backed tests if needed.
  - Make required services and feature flags explicit in the workflow.
- [x] Validate Diesel migrations against a disposable PostgreSQL 18 Alpine
  database, matching the Rust RDB testcontainers environment.
  - Run every migration up, revert all migrations, and run them all again.
  - Reject non-CI database names before the destructive rollback check.
- [x] Validate TypeScript integration-test sources through
  `sh scripts/ci-typecheck.sh`.

```sh
cd tests/integration-tests
pnpm install --frozen-lockfile
pnpm typecheck
```

- [ ] Add a scheduled or manually triggered full HTTP integration-test job.
- [ ] Regenerate `docs/swagger.json` through a checked-in `sh` script into a
  temporary file and fail CI when the checked-in specification differs.
- [ ] Add dependency security checks.
  - Enable Dependabot for Cargo, pnpm, and GitHub Actions.
  - Add `cargo audit` or an agreed `cargo deny` policy.
- [x] Pin third-party GitHub Actions to full commit SHAs.
- [x] Give every workflow an explicit minimal `permissions` block.
- [x] Configure CI concurrency so superseded branch runs are cancelled.

### Acceptance

- [ ] Required CI succeeds on the default branch.
- [ ] A deliberately malformed Rust file, stale OpenAPI file, or failing test
  prevents a pull request from merging.
- [ ] CI does not require production secrets for untrusted pull requests.

## 7. GitHub Actions deployment and container hardening

- [ ] Pin the Docker builder to the selected Rust version.
- [ ] Run the application as a non-root runtime user.
- [ ] Add an image-level `HEALTHCHECK` or document why orchestration-level
  health checks are authoritative.
- [ ] Review the release image for unnecessary files and packages.
- [ ] Decide on the official image registry and immutable image naming scheme.
  - Use an organization-owned registry such as GHCR.
  - Deploy an immutable digest or commit-qualified tag, not `latest`.
- [ ] Add a GitHub Actions deployment workflow.
  - Trigger deployment only after changes land on `main`; never deploy from
    `dev` or a feature branch.
  - Run all required CI checks before building a release image.
  - Build and push the image from GitHub Actions rather than a maintainer's
    machine.
  - Reference an immutable image digest in the deployment step.
  - Use GitHub deployment environments for production approval and audit.
- [ ] Store production secrets in a protected GitHub environment or an
  approved external secret manager.
  - Keep environment-specific non-secret configuration separate from secrets.
  - Prefer short-lived or federated credentials over long-lived credentials.
- [ ] Define the GitHub Actions-to-runtime deployment mechanism without using
  SSH or copying a maintainer-owned `.env` file.
- [ ] Remove fixed bootstrap administrator credentials from production
  migrations, or replace them with a one-time secret provisioning process.
- [ ] Remove the dependency on legacy production credentials at
  `/opt/poprako-s/shared/.env` after migration.
- [ ] Retire the legacy manual deployment implementation after the GitHub
  Actions workflow is verified.
  - Remove `scripts/deploy-release.sh` and
    `scripts/remote-deploy-release.sh`.
  - Remove obsolete manual release recipes from `justfile`.
  - Remove the old `poprako-sr` deployment paths and names.
- [ ] Make deployment rollback-safe.
  - Do not delete the healthy container before the replacement is healthy.
  - Preserve the previous image and runtime configuration.
  - Automatically restore the previous release when the new health check
    fails.
- [ ] Define release retention and cleanup for registry images and deployed
  revisions.
- [ ] Add log, metric, and alert checks to the post-deployment verification.
- [x] Change the root Agent rule so releases must use GitHub Actions rather
  than `just deploy-release`.

### Acceptance

- [ ] A failed deployment leaves or restores a healthy previous version.
- [ ] Production deployment does not require a maintainer machine, SSH, or a
  developer-owned plaintext secret file.
- [ ] A deployed commit can be mapped to an immutable image digest and release
  record.

## 8. Release policy

- [ ] Define the relationship between Cargo version, Git tags, container image
  tags, and GitHub Releases.
- [ ] Choose a versioning policy for the production service.
- [ ] Add a changelog or generate release notes from conventional commits.
- [ ] Define who may create and approve a production release.
- [ ] Build release artifacts only after required CI succeeds.
- [ ] Generate and retain checksums, an SBOM, and provenance for published
  artifacts where practical.
- [ ] Test the release process in a staging environment before the first
  organization-owned production release.

### Acceptance

- [ ] Every production deployment has a tag or release record, source commit,
  image digest, approver, and deployment result.
- [ ] The documented rollback procedure has been exercised successfully.

## 9. Transfer-day procedure

- [ ] Freeze merges and deployments for the transfer window.
- [ ] Confirm the working tree is clean and all intended branches are pushed.
- [ ] Record current repository settings.
  - Default branch and merge methods.
  - Collaborators and permissions.
  - Branch protections or rulesets.
  - Webhooks, deploy keys, Actions secrets, environments, and Pages settings.
- [ ] Back up critical repository and deployment configuration.
- [ ] Confirm the target organization permits repository creation and accepts
  the final repository name.
- [ ] Transfer the repository through GitHub repository settings.
- [ ] Do not recreate the old personal repository at the previous path; doing
  so can break GitHub's redirect.
- [ ] Update local clones after transfer.

```sh
git remote set-url origin git@github.com:poprako-dev/poprako-server.git
git remote -v
```

- [ ] Update external integrations, badges, documentation links, registry
  paths, deployment automation, and bookmarks.

## 10. Organization-side verification

- [ ] Replace direct personal collaborators with organization teams where
  practical.
- [ ] Apply least-privilege base permissions.
- [ ] Configure a ruleset for the default and release branches.
  - Require pull requests.
  - Require approval from someone other than the latest pusher.
  - Require CODEOWNERS where applicable.
  - Require resolved conversations and required CI checks.
  - Block force pushes and branch deletion.
  - Decide whether signed commits and linear history are required.
- [ ] Configure protected deployment environments.
  - Restrict production deployments to approved branches or tags.
  - Require an independent reviewer where the GitHub plan supports it.
  - Disable administrator bypass if appropriate.
- [ ] Re-audit webhooks, deploy keys, secrets, and GitHub App access inherited
  during transfer.
- [ ] Enable the dependency graph, Dependabot alerts and updates, secret
  scanning, and push protection where available.
- [ ] Verify issue assignments, package links, branch protections, and all
  external integrations after transfer.
- [ ] Run CI and perform a staging deployment from the organization-owned
  repository.

### Acceptance

- [ ] Direct pushes to protected branches are rejected.
- [ ] A normal pull request can pass CI, receive review, and merge.
- [ ] A release can deploy through the protected environment without using a
  personal repository or personal registry namespace.

## 11. Final sign-off

- [ ] Repository owner sign-off.
- [ ] Application maintainer sign-off.
- [ ] Infrastructure/deployment owner sign-off.
- [ ] Security contact sign-off.
- [ ] First organization-owned release completed and verified.

## Current audit baseline

The checklist was created from the repository state audited on 2026-08-02.

- `cargo check --workspace --all-features`: passed.
- `sh fmt/run-check.sh`: passed.
- Strict workspace/all-target Clippy with warnings denied: failed with six
  diagnostics.
- Tracked Rust files: 502.
- Rust files at or above 600 lines: 15.
- Migration `up.sql` files: 41.
- Migration `down.sql` files: 41.
- Git tags: 0.
- GitHub Actions workflows: 0.
- The working tree was clean before this checklist was added.
