CREATE TABLE IF NOT EXISTS "t_user_avatar" (
    "f_id"                         TEXT        PRIMARY KEY,
    "f_version"                    BIGINT      NOT NULL,
    "f_is_uploaded"                BOOLEAN,
    "f_hash"                       BYTEA,
    "f_ext"                        TEXT,
    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
