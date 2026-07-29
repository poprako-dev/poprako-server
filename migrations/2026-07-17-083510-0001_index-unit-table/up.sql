CREATE UNIQUE INDEX IF NOT EXISTS "uidx_unit_page_id_index"
    ON "t_unit" ("f_page_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_unit_page_id"
    ON "t_unit" ("f_page_id");
