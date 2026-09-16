CREATE TABLE IF NOT EXISTS "t_unit_save" (
    "f_user_id" TEXT NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,
    "f_page_id" TEXT NOT NULL REFERENCES "t_page" ("f_id") ON DELETE CASCADE,
    "f_save_id" TEXT NOT NULL,
    "f_payload_digest" BYTEA NOT NULL,
    "f_created_unit_ids" JSONB NOT NULL,
    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("f_user_id", "f_page_id", "f_save_id")
);
