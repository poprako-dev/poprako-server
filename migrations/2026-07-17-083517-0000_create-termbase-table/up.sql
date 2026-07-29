CREATE TABLE IF NOT EXISTS "t_termbase" (
    "f_id"          TEXT        PRIMARY KEY,

    "f_team_id"     TEXT        REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_comic_id"    TEXT        REFERENCES "t_comic" ("f_id") ON DELETE RESTRICT,

    "f_name"        TEXT        NOT NULL,
    "f_description" TEXT,

    "f_term_count"  INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"  TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_created_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
