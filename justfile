set dotenv-load := true

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

connect:
    psql ${DATABASE_URL}

check-fix:
    cargo fmt \
        && cargo check \
        && cargo clippy --fix --lib -p poprako-r --allow-dirty -- --no-deps
