CREATE UNIQUE INDEX IF NOT EXISTS "uidx_local_message_processing_topic"
    ON "t_local_message" ("f_topic")
    WHERE "f_status" = 'local_message_status:processing';
