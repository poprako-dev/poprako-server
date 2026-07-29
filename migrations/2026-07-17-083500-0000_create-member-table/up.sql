CREATE TABLE IF NOT EXISTS "t_member" (
    "f_id" TEXT PRIMARY KEY,

    "f_user_id" TEXT NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,
    "f_user_nickname" TEXT NOT NULL,
    "f_team_id" TEXT NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,

    "f_assigned_raw_provider_at" TIMESTAMPTZ,
    "f_assigned_translator_at" TIMESTAMPTZ,
    "f_assigned_proofreader_at" TIMESTAMPTZ,
    "f_assigned_typesetter_at" TIMESTAMPTZ,
    "f_assigned_redrawer_at" TIMESTAMPTZ,
    "f_assigned_reviewer_at" TIMESTAMPTZ,
    "f_assigned_publisher_at" TIMESTAMPTZ,
    "f_assigned_admin_at" TIMESTAMPTZ,
    "f_assigned_bot_at" TIMESTAMPTZ,

    "f_user_last_active_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- For listing team members.
CREATE INDEX IF NOT EXISTS "idx_member_team_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC);
-- For exsitence check of a user in a team.
CREATE UNIQUE INDEX IF NOT EXISTS "uidx_member_user_team"
    ON "t_member" ("f_user_id", "f_team_id");
-- For listing teams of a user.
CREATE INDEX IF NOT EXISTS "idx_member_user_id"
    ON "t_member" ("f_user_id");

-- For listing members with specific role in a team.
CREATE INDEX IF NOT EXISTS "idx_member_team_raw_provider"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_raw_provider_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_translator"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_translator_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_proofreader"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_proofreader_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_typesetter"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_typesetter_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_redrawer"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_redrawer_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_reviewer"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_reviewer_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_publisher"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_publisher_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_admin"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_admin_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_member_team_bot"
    ON "t_member" ("f_team_id")
    WHERE "f_assigned_bot_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_nickname_trgm"
    ON "t_member" USING GIN ("f_user_nickname" gin_trgm_ops);

CREATE INDEX IF NOT EXISTS "idx_member_team_raw_provider_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_raw_provider_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_translator_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_translator_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_proofreader_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_proofreader_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_typesetter_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_typesetter_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_redrawer_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_redrawer_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_reviewer_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_reviewer_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_publisher_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_publisher_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_admin_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_admin_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_bot_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_bot_at" IS NOT NULL;
