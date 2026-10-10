CREATE TABLE IF NOT EXISTS "t_issue" (
    "f_id"              TEXT             PRIMARY KEY,

    "f_page_artwork_id" TEXT             NOT NULL REFERENCES "t_page_artwork" ("f_id") ON DELETE CASCADE,
    "f_index"           INTEGER          NOT NULL,

    "f_variant"         TEXT             NOT NULL,
    "f_layer_path"      TEXT,

    "f_x_coord"         DOUBLE PRECISION,
    "f_y_coord"         DOUBLE PRECISION,
    "f_width"           DOUBLE PRECISION,
    "f_height"          DOUBLE PRECISION,

    "f_note"            TEXT             NOT NULL
);
