CREATE TABLE IF NOT EXISTS "t_announcement" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_team_id"                    TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_user_id"                    TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_title"                      TEXT        NOT NULL,
    "f_content"                    TEXT        NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
