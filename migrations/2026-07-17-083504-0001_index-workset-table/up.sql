CREATE UNIQUE INDEX IF NOT EXISTS "uidx_workset_team_id_index"
    ON "t_workset" ("f_team_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_workset_team_id"
    ON "t_workset" ("f_team_id");

CREATE INDEX IF NOT EXISTS "i_workset_pending_sweep"
    ON "t_workset" ("f_deleted_at", "f_id")
    WHERE "f_deleted_at" IS NOT NULL;
