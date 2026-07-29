DROP INDEX IF EXISTS "idx_local_message_processing_topic";
DROP INDEX IF EXISTS "idx_local_message_pending_topic_visible_created";

CREATE INDEX IF NOT EXISTS "idx_local_message_pending_visible_created"
    ON "t_local_message" ("f_visible_at", "f_created_at")
    WHERE "f_status" = 'local_message_status:pending';
