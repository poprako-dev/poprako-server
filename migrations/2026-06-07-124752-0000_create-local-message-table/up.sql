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

CREATE INDEX IF NOT EXISTS "idx_local_message_claim"
    ON "t_local_message" ("f_topic", "f_status", "f_visible_at", "f_created_at");

CREATE INDEX IF NOT EXISTS "idx_local_message_dead"
    ON "t_local_message" ("f_topic", "f_status", "f_updated_at");

CREATE INDEX IF NOT EXISTS "idx_local_message_completed"
    ON "t_local_message" ("f_topic", "f_status", "f_created_at");
