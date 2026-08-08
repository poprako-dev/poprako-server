CREATE TABLE IF NOT EXISTS "t_comic_archive" (
    "f_id"                  TEXT        PRIMARY KEY,

    "f_team_id"             TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_source_comic_id"     TEXT        NOT NULL UNIQUE,

    "f_archived_payload"    TEXT        NOT NULL,
    "f_archiver_id"         TEXT        NOT NULL,
    "f_created_at"          TIMESTAMPTZ NOT NULL
);
