CREATE TABLE IF NOT EXISTS "t_user" (
    "f_id" TEXT PRIMARY KEY,

    "f_nickname" TEXT NOT NULL UNIQUE,
    "f_qid" TEXT NOT NULL UNIQUE,

    "f_is_sadmin" BOOLEAN NOT NULL DEFAULT FALSE,

    "f_avatar_key" TEXT,
    "f_avatar_source" TEXT,
    "f_avatar_uploaded" BOOLEAN NOT NULL DEFAULT FALSE,

    "f_password_hash" TEXT NOT NULL,

    "f_last_active_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_user_qid"
    ON "t_user" ("f_qid");

