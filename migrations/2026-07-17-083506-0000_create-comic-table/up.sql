CREATE TABLE IF NOT EXISTS "t_comic" (
    "f_id"                  TEXT        PRIMARY KEY,

    "f_workset_id"          TEXT        NOT NULL REFERENCES "t_workset" ("f_id") ON DELETE RESTRICT,
    "f_index"               INTEGER     NOT NULL,

    "f_title"               TEXT        NOT NULL,
    "f_author"              TEXT        NOT NULL,
    "f_description"         TEXT,
    "f_composed_title"      TEXT        NOT NULL DEFAULT '',

    "f_cover_key"           TEXT,
    "f_cover_uploaded"      BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_cover_version"       BIGINT      NOT NULL DEFAULT 0,
    "f_cover_hash"          BYTEA       NOT NULL DEFAULT decode(repeat('00', 32), 'hex'),
    "f_cover_extension"     TEXT        NOT NULL DEFAULT 'png',

    "f_chapter_count"       INTEGER     NOT NULL DEFAULT 0,
    "f_chapter_next_index"  INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"          TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_last_active_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_created_at"          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
