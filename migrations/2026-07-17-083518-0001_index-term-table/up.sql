CREATE UNIQUE INDEX IF NOT EXISTS "uidx_term_termbase_source"
    ON "t_term" ("f_termbase_id", LOWER(BTRIM("f_source")));

CREATE INDEX IF NOT EXISTS "idx_term_termbase_updated"
    ON "t_term" ("f_termbase_id", "f_updated_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_term_source_trgm"
    ON "t_term" USING GIN ("f_source" gin_trgm_ops);
