CREATE TABLE IF NOT EXISTS "t_page" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE RESTRICT,
    "f_index"                      INTEGER     NOT NULL,

    "f_total_unit_count"           INTEGER     NOT NULL DEFAULT 0,
    "f_translated_unit_count"      INTEGER     NOT NULL DEFAULT 0,
    "f_proofread_unit_count"       INTEGER     NOT NULL DEFAULT 0,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
