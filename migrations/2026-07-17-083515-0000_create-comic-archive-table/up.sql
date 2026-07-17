CREATE TABLE IF NOT EXISTS "t_comic_archive" (
    "f_id"                  TEXT        PRIMARY KEY,

    "f_team_id"             TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,

    "f_archived_bytes"      BYTEA       NOT NULL,
    "f_archiver_id"         TEXT        NOT NULL,
    "f_created_at"          TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_comic_archive_team_created_at"
    ON "t_comic_archive" ("f_team_id", "f_created_at" DESC);
