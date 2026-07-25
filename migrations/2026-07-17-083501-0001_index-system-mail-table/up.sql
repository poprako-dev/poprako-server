CREATE INDEX IF NOT EXISTS "idx_system_mail_receiver_created"
    ON "t_system_mail" ("f_receiver_id", "f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_system_mail_unread_receiver_created"
    ON "t_system_mail" ("f_receiver_id", "f_created_at" DESC)
    WHERE "f_read" = FALSE;

CREATE INDEX IF NOT EXISTS "idx_system_mail_read_receiver_created"
    ON "t_system_mail" ("f_receiver_id", "f_created_at" DESC)
    WHERE "f_read" = TRUE;
