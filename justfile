set dotenv-load := true

default:
    just --list

mgr-run:
    diesel migration run

mgr-rev:
    diesel migration revert

mgr-reset:
    diesel migration revert -a

mgr-add name:
    diesel migration generate {{name}}

# This command is only used for creating database.
mgr-setup:
    diesel database setup

mgr-list:
    diesel migration list

mgr-schema:
    diesel print-schema > src/part_impl/repo_rdb/schema.rs

connect:
    psql ${DATABASE_URL}

check-fix:
    cargo fmt \
        && cargo check \
        && cargo clippy --fix --lib -p poprako-r --allow-dirty -- --no-deps

# ── Code Style Checks ──

# Run all 20 code style checks
style:
    bun run .agents/skills/code-style-check/scripts/check-all.js

# Run individual module checks (A-T)
style-a:
    bun run .agents/skills/code-style-check/scripts/check-a-comments.js
style-b:
    bun run .agents/skills/code-style-check/scripts/check-b-abbreviations.js
style-c:
    bun run .agents/skills/code-style-check/scripts/check-c-imports.js
style-d:
    bun run .agents/skills/code-style-check/scripts/check-d-macros.js
style-e:
    bun run .agents/skills/code-style-check/scripts/check-e-format-strings.js
style-f:
    bun run .agents/skills/code-style-check/scripts/check-f-visibility.js
style-g:
    bun run .agents/skills/code-style-check/scripts/check-g-ownership.js
style-h:
    bun run .agents/skills/code-style-check/scripts/check-h-instrument.js
style-i:
    bun run .agents/skills/code-style-check/scripts/check-i-errors.js
style-j:
    bun run .agents/skills/code-style-check/scripts/check-j-i18n.js
style-k:
    bun run .agents/skills/code-style-check/scripts/check-k-ufcs.js
style-l:
    bun run .agents/skills/code-style-check/scripts/check-l-usecase.js
style-m:
    bun run .agents/skills/code-style-check/scripts/check-m-turbofish.js
style-n:
    bun run .agents/skills/code-style-check/scripts/check-n-aggregates.js
style-o:
    bun run .agents/skills/code-style-check/scripts/check-o-query-trait.js
style-p:
    bun run .agents/skills/code-style-check/scripts/check-p-entity.js
style-q:
    bun run .agents/skills/code-style-check/scripts/check-q-infra-query.js
style-r:
    bun run .agents/skills/code-style-check/scripts/check-r-mock.js
style-s:
    bun run .agents/skills/code-style-check/scripts/check-s-api.js
style-t:
    bun run .agents/skills/code-style-check/scripts/check-t-tests.js
