CREATE TABLE IF NOT EXISTS t_issue (
    f_id TEXT PRIMARY KEY,
    f_page_id TEXT NOT NULL REFERENCES t_page(f_id) ON DELETE CASCADE,
    f_index INTEGER NOT NULL CHECK (f_index >= 0),
    f_variant TEXT NOT NULL CHECK (f_variant ~ '[^[:space:]]'),
    f_layer_path TEXT CHECK (f_layer_path ~ '[^[:space:]]'),
    f_x_coord DOUBLE PRECISION,
    f_y_coord DOUBLE PRECISION,
    f_width DOUBLE PRECISION,
    f_height DOUBLE PRECISION,
    f_note TEXT NOT NULL,
    UNIQUE (f_page_id, f_index),
    CHECK (num_nonnulls(f_x_coord, f_y_coord, f_width, f_height) IN (0, 4)),
    CHECK (f_x_coord >= 0 AND f_y_coord >= 0
        AND f_width > 0 AND f_height > 0
        AND f_x_coord + f_width <= 1 AND f_y_coord + f_height <= 1)
);

