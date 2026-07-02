CREATE TABLE IF NOT EXISTS "t_chapter" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_comic_id"                   TEXT        NOT NULL REFERENCES "t_comic" ("f_id") ON DELETE CASCADE,
    "f_is_pinned"                  BOOLEAN     NOT NULL DEFAULT FALSE,

    "f_index"                      INTEGER     NOT NULL,
    "f_subtitle"                   TEXT        NOT NULL,

    "f_page_count"                 INTEGER     NOT NULL DEFAULT 0,
    "f_total_unit_count"           INTEGER     NOT NULL DEFAULT 0,
    "f_translated_unit_count"      INTEGER     NOT NULL DEFAULT 0,
    "f_proofread_unit_count"       INTEGER     NOT NULL DEFAULT 0,

    "f_stages"                     INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"                 TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_chapter_comic_id_index"
    ON "t_chapter" ("f_comic_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_chapter_comic_id_index_desc"
    ON "t_chapter" ("f_comic_id", "f_index" DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_chapter_comic_id_pinned_true"
    ON "t_chapter" ("f_comic_id")
    WHERE "f_is_pinned" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_chapter_creator_id"
    ON "t_chapter" ("f_creator_id");
