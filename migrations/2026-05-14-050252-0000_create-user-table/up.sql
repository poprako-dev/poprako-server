CREATE TABLE IF NOT EXISTS "tbl_user" (
    "id" TEXT PRIMARY KEY,

    "nickname" TEXT NOT NULL UNIQUE,
    "qid" TEXT NOT NULL UNIQUE,

    "is_sadmin" BOOLEAN NOT NULL DEFAULT FALSE,

    "avatar_key" TEXT,
    "avatar_source" TEXT,
    "avatar_uploaded" BOOLEAN NOT NULL DEFAULT FALSE,

    "password_hash" TEXT NOT NULL,

    "last_active_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_user_qid"
    ON "tbl_user" ("qid");
CREATE INDEX IF NOT EXISTS "trgm_idx_user_nickname"
    ON "tbl_user" USING gin ("nickname" gin_trgm_ops);

-- Create super admin directly in database.
INSERT INTO "tbl_user" (
    "id", 
    "qid", 
    "nickname", 
    "password_hash",
    "is_sadmin"
) VALUES (
    'user-00000000-0000-0000-0000-000000000001',
    '123456789',
    'SuperAdmin-OvO',
    '$2a$10$eEEkAsc7h3jdkOyjahdH6OX20w/dHKdGVaH7MNREkh54O57v.E2y2', -- 123456
    TRUE
) ON CONFLICT (id) DO NOTHING;
