-- Source schema: main at b514b4bf, before the Prom reliability branch.
DROP TABLE IF EXISTS public.t_local_message, public.t_obj_prom_task;
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
CREATE TABLE IF NOT EXISTS "t_obj_prom_task" (
    "f_id"                         TEXT        PRIMARY KEY,
    "f_topic"                      TEXT        NOT NULL,
    "f_oper"                       TEXT        NOT NULL,
    "f_obj_id"                     TEXT        NOT NULL,
    "f_version"                    BIGINT      NOT NULL,
    "f_key"                        TEXT        NOT NULL,
    "f_generation"                 BIGINT      NOT NULL,
    "f_status"                     TEXT        NOT NULL,
    "f_visible_at"                 TIMESTAMPTZ NOT NULL,
    "f_retried_count"              BIGINT      NOT NULL DEFAULT 0,
    "f_lease"                      BIGINT      NOT NULL DEFAULT 0,
    "f_error"                      TEXT,
    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS "i_obj_prom_task_poll"
    ON "t_obj_prom_task" (
        "f_status",
        "f_visible_at",
        "f_created_at",
        "f_id"
    );

CREATE INDEX IF NOT EXISTS "i_obj_prom_task_stuck"
    ON "t_obj_prom_task" (
        "f_status",
        "f_updated_at",
        "f_lease"
    );
