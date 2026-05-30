CREATE TABLE IF NOT EXISTS "t_system_mail" (
    "id" TEXT PRIMARY KEY,

    "receiver_id" TEXT NOT NULL REFERENCES "t_user" ("id") ON DELETE CASCADE,
    "title" TEXT NOT NULL,
    "content" TEXT NOT NULL,

    "read" BOOLEAN NOT NULL DEFAULT FALSE,

    "created_at" TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_system_mail_receiver" ON "t_system_mail" ("receiver_id");

CREATE INDEX IF NOT EXISTS "idx_system_mail_unread_receiver_created"
    ON "t_system_mail" ("receiver_id", "created_at" DESC)
    WHERE "read" = FALSE;
