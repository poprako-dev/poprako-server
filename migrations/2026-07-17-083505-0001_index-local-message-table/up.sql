CREATE INDEX IF NOT EXISTS "idx_local_message_status_topic_visible_created"
    ON "t_local_message" ("f_status", "f_topic", "f_visible_at", "f_created_at", "f_id");

CREATE INDEX IF NOT EXISTS "idx_local_message_status_updated"
    ON "t_local_message" ("f_status", "f_updated_at");
