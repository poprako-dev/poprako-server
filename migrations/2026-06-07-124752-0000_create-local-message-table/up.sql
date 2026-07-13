-- Your SQL goes here
CREATE TABLE IF NOT EXISTS "t_local_message" (
    "f_id" TEXT PRIMARY KEY,
    "f_topic" TEXT NOT NULL,
    "f_status" TEXT NOT NULL,
    "f_payload" JSONB NOT NULL,
    "f_last_error" TEXT,
    "f_retried_count" BIGINT NOT NULL DEFAULT 0,
    "f_lease" BIGINT NOT NULL DEFAULT 0,
    "f_visible_at" TIMESTAMPTZ NOT NULL,
    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_local_message_pending_visible_created"
    ON "t_local_message" ("f_visible_at", "f_created_at")
    WHERE "f_status" = 'local_message_status:pending';

CREATE INDEX IF NOT EXISTS "idx_local_message_processing_updated_lease"
    ON "t_local_message" ("f_updated_at", "f_lease")
    WHERE "f_status" = 'local_message_status:processing';

CREATE INDEX IF NOT EXISTS "idx_local_message_dead_updated"
    ON "t_local_message" ("f_updated_at")
    WHERE "f_status" = 'local_message_status:dead';

CREATE INDEX IF NOT EXISTS "idx_local_message_completed_updated"
    ON "t_local_message" ("f_updated_at")
    WHERE "f_status" = 'local_message_status:completed';
