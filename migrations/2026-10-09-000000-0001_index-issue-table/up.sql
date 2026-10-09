CREATE UNIQUE INDEX IF NOT EXISTS "uidx_issue_page_id_index"
    ON "t_issue" ("f_page_id", "f_index");
