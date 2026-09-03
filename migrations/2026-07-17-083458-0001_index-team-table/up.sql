CREATE INDEX IF NOT EXISTS "idx_team_created_at"
    ON "t_team" ("f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "i_team_pending_sweep"
    ON "t_team" ("f_deleted_at", "f_id")
    WHERE "f_deleted_at" IS NOT NULL;
