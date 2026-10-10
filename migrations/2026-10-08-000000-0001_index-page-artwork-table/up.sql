CREATE UNIQUE INDEX IF NOT EXISTS "uidx_page_artwork_chapter_index"
    ON "t_page_artwork" ("f_chapter_id", "f_index");
