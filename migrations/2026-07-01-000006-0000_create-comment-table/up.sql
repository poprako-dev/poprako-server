CREATE TABLE IF NOT EXISTS "t_comment" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_team_id"                    TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_user_id"                    TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_content"                    TEXT        NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_comment_team_id_created_at_desc"
    ON "t_comment" ("f_team_id", "f_created_at" DESC);
