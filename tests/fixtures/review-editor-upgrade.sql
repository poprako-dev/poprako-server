-- Verify additive migration and replay without touching the real business schema.
DO $$
BEGIN
    IF current_database() <> 'db_poprako_ci' THEN
        RAISE EXCEPTION 'This regression requires db_poprako_ci';
    END IF;
END $$;

CREATE SCHEMA ci_review_editor_upgrade;
SET search_path = ci_review_editor_upgrade;
CREATE TABLE t_chapter (f_id TEXT PRIMARY KEY, f_subtitle TEXT NOT NULL);
CREATE TABLE t_page_artwork (f_id TEXT PRIMARY KEY, f_chapter_id TEXT NOT NULL);
CREATE TABLE t_issue (f_id TEXT PRIMARY KEY, f_page_artwork_id TEXT NOT NULL, f_note TEXT NOT NULL);
INSERT INTO t_chapter VALUES ('chapter', 'preserved');
INSERT INTO t_page_artwork VALUES ('page', 'chapter');
INSERT INTO t_issue VALUES ('legacy', 'page', 'preserved note');

\ir ../../scripts/migrate-review-editor.sql

DO $$
BEGIN
    IF (SELECT f_review_generation FROM t_chapter WHERE f_id = 'chapter') <> 0
       OR (SELECT f_review_revision FROM t_chapter WHERE f_id = 'chapter') <> 0
       OR (SELECT f_issue_revision FROM t_page_artwork WHERE f_id = 'page') <> 0 THEN
        RAISE EXCEPTION 'Missing initial review revisions';
    END IF;
END $$;

UPDATE t_chapter SET f_review_generation = 2, f_review_revision = 9;
UPDATE t_page_artwork SET f_issue_revision = 5;

\ir ../../scripts/migrate-review-editor.sql

DO $$
BEGIN
    IF (SELECT f_review_generation FROM t_chapter WHERE f_id = 'chapter') <> 2
       OR (SELECT f_review_revision FROM t_chapter WHERE f_id = 'chapter') <> 9
       OR (SELECT f_issue_revision FROM t_page_artwork WHERE f_id = 'page') <> 5
       OR (SELECT f_subtitle FROM t_chapter WHERE f_id = 'chapter') <> 'preserved'
       OR (SELECT f_note FROM t_issue WHERE f_id = 'legacy') <> 'preserved note' THEN
        RAISE EXCEPTION 'Upgrade replay changed existing review data';
    END IF;
END $$;

RESET search_path;
DROP SCHEMA ci_review_editor_upgrade CASCADE;
