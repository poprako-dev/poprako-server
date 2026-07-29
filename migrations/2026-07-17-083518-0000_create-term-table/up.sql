CREATE TABLE IF NOT EXISTS "t_term" (
    "f_id"          TEXT        PRIMARY KEY,

    "f_termbase_id" TEXT        NOT NULL REFERENCES "t_termbase" ("f_id") ON DELETE RESTRICT,

    "f_source"      TEXT        NOT NULL,
    "f_targets"     TEXT[]      NOT NULL,
    "f_comment"     TEXT,

    "f_creator_id"  TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_created_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "chk_term_targets_nonempty"
        CHECK (CARDINALITY("f_targets") > 0),
    CONSTRAINT "chk_term_targets_nonnull"
        CHECK (ARRAY_POSITION("f_targets", NULL) IS NULL)
);
