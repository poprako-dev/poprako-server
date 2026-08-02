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
    diesel print-schema > src/part_impl/repo/rdb_impl/schema.rs

connect:
    psql ${DATABASE_URL}

check-fix:
    cargo fmt \
        && cargo check \
        && cargo clippy --fix --lib -p poprako-server --allow-dirty -- --no-deps

prod-build:
    scripts/docker-build-prod.sh

prod-run:
    scripts/local-run-release.sh

prod-stop:
    scripts/local-stop-release.sh

prod-ci-build:
    scripts/ci-build-prod.sh

deploy-release:
    sh scripts/deploy-release.sh

# Generate swagger.json from the annotated OpenAPI spec.
swagger:
    cargo run -p poprako-swagger > docs/swagger.json

# Optional local wrapper for the repository's CI validation entry point.
fmt:
    sh scripts/ci-check.sh
