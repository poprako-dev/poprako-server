CREATE TABLE IF NOT EXISTS "t_page" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE RESTRICT,
    "f_index"                      INTEGER     NOT NULL,

    "f_image_key"                  TEXT,
    "f_image_uploaded"             BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_image_version"              BIGINT      NOT NULL DEFAULT 0,

    "f_total_unit_count"           INTEGER     NOT NULL DEFAULT 0,
    "f_translated_unit_count"      INTEGER     NOT NULL DEFAULT 0,
    "f_proofread_unit_count"       INTEGER     NOT NULL DEFAULT 0,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_page_chapter_id_index"
    ON "t_page" ("f_chapter_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_page_chapter_id"
    ON "t_page" ("f_chapter_id");
