CREATE INDEX IF NOT EXISTS "idx_chapter_workflow_record_chapter_created_id_desc"
    ON "t_chapter_workflow_record" ("f_chapter_id", "f_created_at" DESC, "f_id" DESC);
