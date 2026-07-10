CREATE TABLE IF NOT EXISTS "t_comic" (
    "f_id"                  TEXT        PRIMARY KEY,

    "f_workset_id"          TEXT        NOT NULL REFERENCES "t_workset" ("f_id") ON DELETE RESTRICT,
    "f_index"               INTEGER     NOT NULL,

    "f_title"               TEXT        NOT NULL,
    "f_author"              TEXT        NOT NULL,
    "f_description"         TEXT,

    "f_cover_key"           TEXT,
    "f_cover_uploaded"      BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_cover_version"       BIGINT      NOT NULL DEFAULT 0,

    "f_chapter_count"       INTEGER     NOT NULL DEFAULT 0,
    "f_chapter_next_index"  INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"          TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_last_active_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_created_at"          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_comic_workset_id_index"
    ON "t_comic" ("f_workset_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_comic_workset_id"
    ON "t_comic" ("f_workset_id");

CREATE INDEX IF NOT EXISTS "idx_comic_creator_id"
    ON "t_comic" ("f_creator_id");

CREATE TABLE IF NOT EXISTS "t_archived_comic" (
    "f_id"                  TEXT        PRIMARY KEY,
    "f_archived_bytes"      BYTEA       NOT NULL,
    "f_archiver_id"         TEXT        NOT NULL,
    "f_created_at"          TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS "t_archived_chapter" (
    "f_id"                  TEXT        PRIMARY KEY,
    "f_archived_bytes"      BYTEA       NOT NULL,
    "f_archiver_id"         TEXT        NOT NULL,
    "f_created_at"          TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS "t_archived_translation" (
    "f_id"                  TEXT        PRIMARY KEY,
    "f_archived_bytes"      BYTEA       NOT NULL,
    "f_archiver_id"         TEXT        NOT NULL,
    "f_created_at"          TIMESTAMPTZ NOT NULL
);
