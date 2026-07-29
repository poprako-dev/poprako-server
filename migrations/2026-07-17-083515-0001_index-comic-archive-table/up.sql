CREATE INDEX IF NOT EXISTS "idx_comic_archive_team_created_at"
    ON "t_comic_archive" ("f_team_id", "f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_comic_archive_created_at_team"
    ON "t_comic_archive" ("f_created_at", "f_team_id");
