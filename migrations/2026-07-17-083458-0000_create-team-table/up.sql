CREATE TABLE IF NOT EXISTS "t_team" (
    "f_id" TEXT PRIMARY KEY,

    "f_name" TEXT NOT NULL UNIQUE,
    "f_description" TEXT,

    "f_workset_next_index" INTEGER NOT NULL DEFAULT 0,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
