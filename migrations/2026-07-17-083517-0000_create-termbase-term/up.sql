CREATE TABLE IF NOT EXISTS "t_termbase" (
    "f_id"          TEXT        PRIMARY KEY,

    "f_team_id"     TEXT        REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,
    "f_comic_id"    TEXT        REFERENCES "t_comic" ("f_id") ON DELETE RESTRICT,

    "f_name"        TEXT        NOT NULL,
    "f_description" TEXT,

    "f_term_count"  INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"  TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_created_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "chk_termbase_scope"
        CHECK (("f_team_id" IS NOT NULL) <> ("f_comic_id" IS NOT NULL)),
    CONSTRAINT "chk_termbase_term_count"
        CHECK ("f_term_count" >= 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_termbase_team_name"
    ON "t_termbase" ("f_team_id", LOWER(BTRIM("f_name")))
    WHERE "f_team_id" IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_termbase_comic_name"
    ON "t_termbase" ("f_comic_id", LOWER(BTRIM("f_name")))
    WHERE "f_comic_id" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_termbase_team_updated"
    ON "t_termbase" ("f_team_id", "f_updated_at" DESC)
    WHERE "f_team_id" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_termbase_comic_updated"
    ON "t_termbase" ("f_comic_id", "f_updated_at" DESC)
    WHERE "f_comic_id" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_termbase_name_trgm"
    ON "t_termbase" USING GIN ("f_name" gin_trgm_ops);

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

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_term_termbase_source"
    ON "t_term" ("f_termbase_id", LOWER(BTRIM("f_source")));

CREATE INDEX IF NOT EXISTS "idx_term_termbase_updated"
    ON "t_term" ("f_termbase_id", "f_updated_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_term_source_trgm"
    ON "t_term" USING GIN ("f_source" gin_trgm_ops);
