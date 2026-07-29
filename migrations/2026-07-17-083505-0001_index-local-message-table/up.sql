CREATE INDEX IF NOT EXISTS "idx_local_message_pending_topic_visible_created"
    ON "t_local_message" ("f_topic", "f_visible_at", "f_created_at", "f_id")
    WHERE "f_status" = 'local_message_status:pending';

CREATE INDEX IF NOT EXISTS "idx_local_message_processing_updated_lease"
    ON "t_local_message" ("f_updated_at", "f_lease")
    WHERE "f_status" = 'local_message_status:processing';

CREATE INDEX IF NOT EXISTS "idx_local_message_processing_topic"
    ON "t_local_message" ("f_topic")
    WHERE "f_status" = 'local_message_status:processing';

CREATE INDEX IF NOT EXISTS "idx_local_message_dead_updated"
    ON "t_local_message" ("f_updated_at")
    WHERE "f_status" = 'local_message_status:dead';

CREATE INDEX IF NOT EXISTS "idx_local_message_completed_updated"
    ON "t_local_message" ("f_updated_at")
    WHERE "f_status" = 'local_message_status:completed';
