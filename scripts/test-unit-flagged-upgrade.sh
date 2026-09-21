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

test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT
trap 'exit 1' INT TERM

sql() {
    psql "$DATABASE_URL" -X -q -A -t -v ON_ERROR_STOP=1 "$@"
}

schema_contract() {
    sql -c "SELECT a.attname, format_type(a.atttypid, a.atttypmod),
        a.attnotnull, pg_get_expr(d.adbin, d.adrelid)
        FROM pg_attribute a LEFT JOIN pg_attrdef d
          ON d.adrelid = a.attrelid AND d.adnum = a.attnum
        WHERE a.attrelid = 'public.t_unit'::regclass
          AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attname"
}

snapshot() {
    sql -c "SELECT to_jsonb(u) FROM t_unit u
        WHERE f_id LIKE 'flagged-upgrade-%' ORDER BY f_id"
    schema_contract
}

schema_contract > "$test_dir/baseline"

# Simulate a populated old database, including a hidden Unit.
sql <<'SQL'
ALTER TABLE t_unit DROP COLUMN f_is_flagged;
INSERT INTO t_workset (f_id, f_team_id, f_index, f_name)
SELECT 'flagged-upgrade-workset', f_id, 12345, 'upgrade fixture'
FROM t_team ORDER BY f_id LIMIT 1;
INSERT INTO t_comic (f_id, f_workset_id, f_index, f_title, f_author, f_creator_id)
SELECT 'flagged-upgrade-comic', 'flagged-upgrade-workset', 0, 'fixture', 'fixture', f_id
FROM t_user ORDER BY f_id LIMIT 1;
INSERT INTO t_chapter (f_id, f_comic_id, f_index, f_subtitle, f_creator_id)
SELECT 'flagged-upgrade-chapter', 'flagged-upgrade-comic', 0, 'fixture', f_id
FROM t_user ORDER BY f_id LIMIT 1;
INSERT INTO t_page (f_id, f_chapter_id, f_index)
VALUES ('flagged-upgrade-page', 'flagged-upgrade-chapter', 0);
INSERT INTO t_unit (f_id, f_page_id, f_next_id, f_x_coord, f_y_coord, f_translated_text)
VALUES ('flagged-upgrade-visible', 'flagged-upgrade-page', NULL, 1, 2, 'retained');
INSERT INTO t_unit (f_id, f_page_id, f_next_id, f_hidden_at, f_x_coord, f_y_coord)
VALUES ('flagged-upgrade-hidden', 'flagged-upgrade-page', 'flagged-upgrade-visible', NOW(), 3, 4);
SQL

sql -c "SELECT to_jsonb(u) FROM t_unit u WHERE f_id LIKE 'flagged-upgrade-%' ORDER BY f_id" > "$test_dir/old-data"
sql -f scripts/migrate-unit-flagged.sql
sql -c "SELECT to_jsonb(u) - 'f_is_flagged' FROM t_unit u WHERE f_id LIKE 'flagged-upgrade-%' ORDER BY f_id" > "$test_dir/upgraded-data"
diff -u "$test_dir/old-data" "$test_dir/upgraded-data"
schema_contract > "$test_dir/upgraded-schema"
diff -u "$test_dir/baseline" "$test_dir/upgraded-schema"
[ "$(sql -c "SELECT count(*) FROM t_unit WHERE f_id LIKE 'flagged-upgrade-%' AND NOT f_is_flagged")" = 2 ]

sql -c "UPDATE t_unit SET f_is_flagged = TRUE WHERE f_id = 'flagged-upgrade-visible'"
snapshot > "$test_dir/before-replay"
sql -f scripts/migrate-unit-flagged.sql
snapshot > "$test_dir/after-replay"
diff -u "$test_dir/before-replay" "$test_dir/after-replay"

# Existing incompatible columns must be rejected without changing data or schema.
for definition in 'BOOLEAN DEFAULT FALSE' 'BOOLEAN NOT NULL DEFAULT TRUE' 'INTEGER NOT NULL DEFAULT 0'; do
    sql -c "ALTER TABLE t_unit DROP COLUMN f_is_flagged;
        ALTER TABLE t_unit ADD COLUMN f_is_flagged $definition"
    snapshot > "$test_dir/before-failure"
    if sql -f scripts/migrate-unit-flagged.sql > "$test_dir/failure" 2>&1; then
        echo "upgrade accepted incompatible column: $definition" >&2
        exit 1
    fi
    grep -F 'must be BOOLEAN NOT NULL DEFAULT FALSE' "$test_dir/failure" >/dev/null
    snapshot > "$test_dir/after-failure"
    diff -u "$test_dir/before-failure" "$test_dir/after-failure"
done

sql -c 'ALTER TABLE t_unit DROP COLUMN f_is_flagged'
sql -f scripts/migrate-unit-flagged.sql
sql <<'SQL'
DELETE FROM t_unit WHERE f_page_id = 'flagged-upgrade-page';
DELETE FROM t_page WHERE f_id = 'flagged-upgrade-page';
DELETE FROM t_chapter WHERE f_id = 'flagged-upgrade-chapter';
DELETE FROM t_comic WHERE f_id = 'flagged-upgrade-comic';
DELETE FROM t_workset WHERE f_id = 'flagged-upgrade-workset';
SQL

echo 'Unit flagged upgrade checks passed'
