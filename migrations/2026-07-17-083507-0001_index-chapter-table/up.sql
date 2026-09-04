CREATE UNIQUE INDEX IF NOT EXISTS "uidx_chapter_comic_id_index"
    ON "t_chapter" ("f_comic_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_chapter_comic_id_index_desc"
    ON "t_chapter" ("f_comic_id", "f_index" DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_chapter_comic_id_pinned_true"
    ON "t_chapter" ("f_comic_id")
    WHERE "f_is_pinned" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_chapter_creator_id"
    ON "t_chapter" ("f_creator_id");

CREATE INDEX IF NOT EXISTS "i_chapter_pending_sweep"
    ON "t_chapter" ("f_deleted_at", "f_id")
    WHERE "f_deleted_at" IS NOT NULL;
