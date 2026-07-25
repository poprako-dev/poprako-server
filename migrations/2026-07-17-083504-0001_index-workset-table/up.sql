CREATE UNIQUE INDEX IF NOT EXISTS "uidx_workset_team_id_index"
    ON "t_workset" ("f_team_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_workset_team_id"
    ON "t_workset" ("f_team_id");
