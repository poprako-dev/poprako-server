CREATE UNIQUE INDEX IF NOT EXISTS "uidx_page_source_chapter_id_index"
    ON "t_page_source" ("f_chapter_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_page_source_chapter_id"
    ON "t_page_source" ("f_chapter_id");
