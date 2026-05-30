CREATE TABLE IF NOT EXISTS "t_team" (
    "f_id" TEXT PRIMARY KEY,

    "f_name" TEXT NOT NULL UNIQUE,
    "f_description" TEXT,

    "f_avatar_key" TEXT,
    "f_avatar_uploaded" BOOLEAN NOT NULL DEFAULT FALSE,

    "f_workset_next_index" INTEGER NOT NULL DEFAULT 0,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create a default team directly in database.
INSERT INTO "t_team" (
    "f_id",
    "f_name",
    "f_description"
) VALUES (
    'team-00000000-0000-0000-0000-000000000001',
    'PRTS 汉化组',
    '测测你的'
) ON CONFLICT ("f_id") DO NOTHING;
