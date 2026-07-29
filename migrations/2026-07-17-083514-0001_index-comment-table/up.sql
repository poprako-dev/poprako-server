CREATE INDEX IF NOT EXISTS "idx_comment_team_id_created_at_desc"
    ON "t_comment" ("f_team_id", "f_created_at" DESC);
