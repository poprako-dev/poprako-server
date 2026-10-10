CREATE UNIQUE INDEX IF NOT EXISTS "uidx_issue_page_artwork_index"
    ON "t_issue" ("f_page_artwork_id", "f_index");
