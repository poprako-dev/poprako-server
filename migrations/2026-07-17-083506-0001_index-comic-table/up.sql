CREATE UNIQUE INDEX IF NOT EXISTS "uidx_comic_workset_id_index"
    ON "t_comic" ("f_workset_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_comic_workset_id"
    ON "t_comic" ("f_workset_id");

CREATE INDEX IF NOT EXISTS "idx_comic_creator_id"
    ON "t_comic" ("f_creator_id");

CREATE INDEX IF NOT EXISTS "idx_comic_composed_title_trgm"
    ON "t_comic" USING GIN ("f_composed_title" gin_trgm_ops);

CREATE INDEX IF NOT EXISTS "idx_comic_workset_last_active"
    ON "t_comic" ("f_workset_id", "f_last_active_at" DESC, "f_index" ASC);

CREATE INDEX IF NOT EXISTS "i_comic_pending_sweep"
    ON "t_comic" ("f_deleted_at", "f_id")
    WHERE "f_deleted_at" IS NOT NULL;
