CREATE TABLE IF NOT EXISTS "t_page_artwork" (
    "f_id"         TEXT        PRIMARY KEY,

    "f_chapter_id" TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE CASCADE,
    "f_index"      INTEGER     NOT NULL,

    "f_raw_ident"  TEXT,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
