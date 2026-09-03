CREATE TABLE IF NOT EXISTS "t_comic" (
    "f_id"                  TEXT        PRIMARY KEY,

    "f_workset_id"          TEXT        NOT NULL REFERENCES "t_workset" ("f_id") ON DELETE RESTRICT,
    "f_index"               INTEGER     NOT NULL,

    "f_title"               TEXT        NOT NULL,
    "f_author"              TEXT        NOT NULL,
    "f_description"         TEXT,
    "f_composed_title"      TEXT        NOT NULL DEFAULT '',

    "f_chapter_count"       INTEGER     NOT NULL DEFAULT 0,
    "f_chapter_next_index"  INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"          TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_last_active_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_archived_at"         TIMESTAMPTZ,
    "f_deleted_at"          TIMESTAMPTZ,
    "f_created_at"          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
