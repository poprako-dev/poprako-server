CREATE TABLE IF NOT EXISTS "t_page" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE RESTRICT,
    "f_index"                      INTEGER     NOT NULL,

    "f_image_key"                  TEXT,
    "f_image_uploaded"             BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_image_version"              BIGINT      NOT NULL DEFAULT 0,
    "f_image_hash"                 BYTEA       NOT NULL,
    "f_image_byte_length"          BIGINT      NOT NULL,
    "f_image_extension"            TEXT        NOT NULL,

    "f_total_unit_count"           INTEGER     NOT NULL DEFAULT 0,
    "f_translated_unit_count"      INTEGER     NOT NULL DEFAULT 0,
    "f_proofread_unit_count"       INTEGER     NOT NULL DEFAULT 0,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CHECK (octet_length("f_image_hash") = 32),
    CHECK ("f_image_byte_length" BETWEEN 1 AND 20971520)
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_page_chapter_id_index"
    ON "t_page" ("f_chapter_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_page_chapter_id"
    ON "t_page" ("f_chapter_id");
