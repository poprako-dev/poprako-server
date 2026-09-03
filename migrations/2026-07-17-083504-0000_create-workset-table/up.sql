CREATE TABLE IF NOT EXISTS "t_workset" (
    "f_id"              TEXT        PRIMARY KEY,

    "f_team_id"         TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_index"           INTEGER     NOT NULL,

    "f_name"            TEXT        NOT NULL,
    "f_description"     TEXT,

    "f_comic_count"     INTEGER     NOT NULL DEFAULT 0,
    "f_comic_next_index" INTEGER     NOT NULL DEFAULT 0,

    "f_deleted_at"      TIMESTAMPTZ,
    "f_created_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
