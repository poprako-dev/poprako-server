CREATE TABLE IF NOT EXISTS "t_local_message" (
    "f_id" TEXT PRIMARY KEY,
    "f_topic" TEXT NOT NULL,
    "f_status" TEXT NOT NULL,
    "f_payload" JSONB NOT NULL,
    "f_last_error" TEXT,
    "f_retried_count" BIGINT NOT NULL DEFAULT 0,
    "f_claim_token" UUID,
    "f_visible_at" TIMESTAMPTZ NOT NULL,
    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "ck_local_message_claim_token" CHECK (
        ("f_status" = 'local_message_status:processing') = ("f_claim_token" IS NOT NULL)
    )
);
