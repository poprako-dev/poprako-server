CREATE TABLE IF NOT EXISTS "t_user" (
    "f_id" TEXT PRIMARY KEY,

    "f_nickname" TEXT NOT NULL UNIQUE,
    "f_qid" TEXT NOT NULL UNIQUE,

    "f_is_sadmin" BOOLEAN NOT NULL DEFAULT FALSE,

    "f_avatar_key" TEXT,
    "f_avatar_source" TEXT,
    "f_avatar_uploaded" BOOLEAN NOT NULL DEFAULT FALSE,
    "f_avatar_version" BIGINT NOT NULL DEFAULT 0,
    "f_avatar_hash" BYTEA NOT NULL DEFAULT decode(repeat('00', 32), 'hex'),
    "f_avatar_extension" TEXT NOT NULL DEFAULT 'png',

    "f_password_hash" TEXT NOT NULL,

    "f_last_active_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
