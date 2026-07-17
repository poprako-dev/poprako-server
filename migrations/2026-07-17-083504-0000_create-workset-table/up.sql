CREATE TABLE IF NOT EXISTS "t_workset" (
    "f_id"              TEXT        PRIMARY KEY,

    "f_team_id"         TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_index"           INTEGER     NOT NULL,

    "f_name"            TEXT        NOT NULL,
    "f_description"     TEXT,

    "f_comic_count"     INTEGER     NOT NULL DEFAULT 0,
    "f_comic_next_index" INTEGER     NOT NULL DEFAULT 0,

    "f_created_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_workset_team_id_index"
    ON "t_workset" ("f_team_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_workset_team_id"
    ON "t_workset" ("f_team_id");
