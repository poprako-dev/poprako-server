CREATE TABLE IF NOT EXISTS "t_team" (
    "f_id" TEXT PRIMARY KEY,

    "f_name" TEXT NOT NULL UNIQUE,
    "f_description" TEXT,

    "f_avatar_key" TEXT,
    "f_avatar_uploaded" BOOLEAN NOT NULL DEFAULT FALSE,
    "f_avatar_version" BIGINT NOT NULL DEFAULT 0,
    "f_avatar_hash" BYTEA NOT NULL DEFAULT decode(repeat('00', 32), 'hex'),
    "f_avatar_extension" TEXT NOT NULL DEFAULT 'png',

    "f_workset_next_index" INTEGER NOT NULL DEFAULT 0,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CHECK (octet_length("f_avatar_hash") = 32)
);

CREATE INDEX IF NOT EXISTS "idx_team_created_at"
    ON "t_team" ("f_created_at" DESC);
