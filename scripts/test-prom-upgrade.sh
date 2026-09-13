#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

[ "${CI_MIGRATION_DATABASE:-}" = 1 ] || {
    echo 'CI_MIGRATION_DATABASE=1 is required' >&2
    exit 1
}
case "${DATABASE_URL:-}" in
    postgres://*/db_poprako_ci | postgresql://*/db_poprako_ci) ;;
    *) echo 'DATABASE_URL must target disposable db_poprako_ci' >&2; exit 1 ;;
esac

upgrade_root=scripts/prom-upgrade
test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT
trap 'exit 1' INT TERM

sql() {
    psql "$DATABASE_URL" -X -q -A -t -v ON_ERROR_STOP=1 "$@"
}

reset_source() {
    sql -f "$upgrade_root/tests/source.sql"
    sql -f "$upgrade_root/tests/data.sql"
}

snapshot() {
    sql -c "SELECT 'local', to_jsonb(t) FROM t_local_message t ORDER BY f_id;
        SELECT 'object', to_jsonb(t) FROM t_obj_prom_task t ORDER BY f_id;"
    sql -f "$upgrade_root/tests/schema.sql"
}

expect_failure() {
    expected_message=$1
    snapshot > "$test_dir/before"
    if sql -f "$upgrade_root/apply.sql" > "$test_dir/failure" 2>&1; then
        echo "expected upgrade to reject: $expected_message" >&2
        exit 1
    fi
    if ! grep -F -q "$expected_message" "$test_dir/failure"; then
        cat "$test_dir/failure" >&2
        exit 1
    fi
    snapshot > "$test_dir/after"
    diff -u "$test_dir/before" "$test_dir/after"
}

reset_source
sql -c "UPDATE t_local_message SET f_status = 'local_message_status:processing' WHERE f_id = 'chapter'"
expect_failure 'Unresolved processing tasks'
if sql -f "$upgrade_root/preflight.sql" > "$test_dir/preflight" 2>&1; then
    echo 'preflight must reject unresolved processing tasks' >&2
    exit 1
fi

reset_source
sql -c "UPDATE t_obj_prom_task SET f_status = 'obj_prom_status:processing' WHERE f_id = 'object-pending'"
expect_failure 'Unresolved processing tasks'

reset_source
sql -c "UPDATE t_local_message SET f_payload = f_payload #- '{AdvanceRawProvide,actor_user_id}' WHERE f_id = 'chapter'"
expect_failure 'Invalid pending payload or missing actor_user_id'

reset_source
sql -c "UPDATE t_local_message SET f_payload = jsonb_set(f_payload, '{PurgeExpiredInvitation,Assignment}', '{\"invitation_id\":\"unexpected\"}') WHERE f_id = 'member'"
expect_failure 'Invalid pending payload or missing actor_user_id'

reset_source
sql -c "UPDATE t_local_message SET f_status = 'unknown' WHERE f_id = 'chapter'"
expect_failure 'Unknown queue statuses'

reset_source
sql -c 'ALTER TABLE t_local_message ADD COLUMN f_claim_token UUID'
expect_failure 'mixed schemas are unsupported'

reset_source
sql -c 'CREATE INDEX idx_local_message_status_updated ON t_local_message (f_id)'
expect_failure 'Prom indexes do not match'

reset_source
sql -f "$upgrade_root/preflight.sql" > "$test_dir/preflight"
sql -f "$upgrade_root/apply.sql"
sql -f "$upgrade_root/verify.sql" > "$test_dir/verify"
sql -f "$upgrade_root/tests/assert-data.sql"

# A rerun must preserve newly claimed attempts, not reset or replace their tokens.
sql -c "UPDATE t_local_message SET f_status = 'local_message_status:processing', f_claim_token = gen_random_uuid() WHERE f_id = 'chapter';
    UPDATE t_obj_prom_task SET f_status = 'obj_prom_status:processing', f_claim_token = gen_random_uuid() WHERE f_id = 'object-pending';"
snapshot > "$test_dir/claimed"
sql -f "$upgrade_root/apply.sql"
snapshot > "$test_dir/repeated"
diff -u "$test_dir/claimed" "$test_dir/repeated"

# Reproduce the deployment's full baseline replay against the upgraded database.
{
    printf 'BEGIN;\n'
    for migration_dir in migrations/*; do
        cat "$migration_dir/up.sql"
        printf '\n'
    done
    printf 'COMMIT;\n'
} > "$test_dir/replay.sql"
sql -f "$test_dir/replay.sql" > "$test_dir/replay.log"
sql -f "$test_dir/replay.sql" >> "$test_dir/replay.log"
snapshot > "$test_dir/replayed"
diff -u "$test_dir/claimed" "$test_dir/replayed"
sql -f "$upgrade_root/verify.sql" > "$test_dir/verify"
sql -f "$upgrade_root/tests/schema.sql" > "$test_dir/upgraded-schema"

# Ensure upgrading matches fresh creation without relying on historical ALTERs.
sql -c 'DROP TABLE t_local_message, t_obj_prom_task'
for migration in \
    migrations/2026-07-17-083505-0000_create-local-message-table \
    migrations/2026-07-17-083505-0001_index-local-message-table \
    migrations/2026-08-28-235000-0000_create-obj-prom-task-table \
    migrations/2026-08-28-235001-0000_index-obj-prom-task-table; do
    sql -f "$migration/up.sql"
done
sql -f "$upgrade_root/tests/schema.sql" > "$test_dir/fresh-schema"
diff -u "$test_dir/upgraded-schema" "$test_dir/fresh-schema"
sql -f "$upgrade_root/apply.sql"

sql -c 'ALTER TABLE t_local_message DROP CONSTRAINT ck_local_message_claim_token;
    ALTER TABLE t_local_message ADD CONSTRAINT ck_local_message_claim_token CHECK (true)'
expect_failure 'Prom claim-token constraints are missing, unvalidated or incorrect'
sql -c "ALTER TABLE t_local_message DROP CONSTRAINT ck_local_message_claim_token;
    ALTER TABLE t_local_message ADD CONSTRAINT ck_local_message_claim_token CHECK (
        (f_status = 'local_message_status:processing') = (f_claim_token IS NOT NULL))"

# Complete the repository's required apply -> revert-all -> apply validation.
sh scripts/ci-migration-check.sh > "$test_dir/migration-check.log" 2>&1 || {
    cat "$test_dir/migration-check.log" >&2
    exit 1
}
printf '%s\n' 'Prom manual upgrade: all checks passed'
