CREATE TABLE IF NOT EXISTS "t_chapter_workflow_record" (
    "f_id"              TEXT        PRIMARY KEY,
    "f_chapter_id"      TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE RESTRICT,
    "f_actor_user_id"   TEXT,
    "f_kind"            TEXT        NOT NULL,
    "f_payload"         JSONB       NOT NULL CHECK (jsonb_typeof("f_payload") = 'object'),
    "f_created_at"      TIMESTAMPTZ NOT NULL
);
