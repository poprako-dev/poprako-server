CREATE TABLE IF NOT EXISTS "t_system_mail" (
    "f_id" TEXT PRIMARY KEY,

    "f_receiver_id" TEXT NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,
    "f_title" TEXT NOT NULL,
    "f_content" TEXT NOT NULL,

    "f_read" BOOLEAN NOT NULL DEFAULT FALSE,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_system_mail_receiver" ON "t_system_mail" ("f_receiver_id");

CREATE INDEX IF NOT EXISTS "idx_system_mail_unread_receiver_created"
    ON "t_system_mail" ("f_receiver_id", "f_created_at" DESC)
    WHERE "f_read" = FALSE;
