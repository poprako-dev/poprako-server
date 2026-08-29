CREATE TABLE IF NOT EXISTS "t_obj_prom_task" (
    "f_id"                         TEXT        PRIMARY KEY,
    "f_topic"                      TEXT        NOT NULL,
    "f_oper"                       TEXT        NOT NULL,
    "f_obj_id"                     TEXT        NOT NULL,
    "f_version"                    BIGINT      NOT NULL,
    "f_generation"                 BIGINT      NOT NULL,
    "f_status"                     TEXT        NOT NULL,
    "f_visible_at"                 TIMESTAMPTZ NOT NULL,
    "f_retried_count"              BIGINT      NOT NULL DEFAULT 0,
    "f_lease"                      BIGINT      NOT NULL DEFAULT 0,
    "f_error"                      TEXT,
    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
