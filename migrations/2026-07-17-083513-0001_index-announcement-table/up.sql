CREATE INDEX IF NOT EXISTS "idx_announcement_team_id_created_at_desc"
    ON "t_announcement" ("f_team_id", "f_created_at" DESC);
