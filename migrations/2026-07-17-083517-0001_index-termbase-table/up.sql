CREATE UNIQUE INDEX IF NOT EXISTS "uidx_termbase_team_name"
    ON "t_termbase" ("f_team_id", LOWER(BTRIM("f_name")))
    WHERE "f_team_id" IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_termbase_comic_name"
    ON "t_termbase" ("f_comic_id", LOWER(BTRIM("f_name")))
    WHERE "f_comic_id" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_termbase_team_updated"
    ON "t_termbase" ("f_team_id", "f_updated_at" DESC)
    WHERE "f_team_id" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_termbase_comic_updated"
    ON "t_termbase" ("f_comic_id", "f_updated_at" DESC)
    WHERE "f_comic_id" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_termbase_name_trgm"
    ON "t_termbase" USING GIN ("f_name" gin_trgm_ops);
