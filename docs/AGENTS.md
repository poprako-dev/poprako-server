<!-- OMX:AGENTS-INIT:MANAGED -->
<!-- Parent: ../AGENTS.md -->

# docs

Keep only operational documentation that reflects the checked-in implementation.
Delete one-off plans, completed migration notes, stale backlogs, and generated
artifacts without an active maintainer or regeneration path.

## Current documents

- `organization-transfer-readiness.md` tracks the repository and deployment
  work required before and immediately after transfer to the official
  organization.
- `how-to-implement-api-http.md` defines active Axum, router, and OpenAPI work.
- `unit-save-api.md` defines the public page-unit save contract.
- `swagger.json` is a checked-in generated artifact. Regenerate it with
  `cargo run -p poprako-swagger > docs/swagger.json`; do not edit it by hand.

Integration-test documentation belongs in `tests/integration-tests/` and must
track the TypeScript suite there. Do not duplicate implementation plans here.

<!-- OMX:AGENTS-INIT:MANUAL:START -->

## Local Notes

- Source paths, command examples, and API contracts must point to active code.
- Delete completed implementation checklists instead of preserving misleading
  historical state.
<!-- OMX:AGENTS-INIT:MANUAL:END -->
