CREATE TABLE IF NOT EXISTS "t_unit" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_page_id"                    TEXT        NOT NULL REFERENCES "t_page" ("f_id") ON DELETE RESTRICT,
    "f_index"                      INTEGER     NOT NULL,

    "f_is_bubble"                  BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_is_proofread"               BOOLEAN     NOT NULL DEFAULT FALSE,

    "f_x_coord"                    DOUBLE PRECISION NOT NULL,
    "f_y_coord"                    DOUBLE PRECISION NOT NULL,

    "f_translated_text"            TEXT,
    "f_last_translator_id"         TEXT REFERENCES "t_user" ("f_id") ON DELETE SET NULL,

    "f_proofread_text"             TEXT,
    "f_last_proofreader_id"        TEXT REFERENCES "t_user" ("f_id") ON DELETE SET NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
