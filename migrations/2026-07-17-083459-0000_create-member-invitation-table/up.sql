CREATE TABLE IF NOT EXISTS "t_member_invitation" (
    "f_id" TEXT PRIMARY KEY,

    "f_inviter_id" TEXT NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,
    "f_team_id" TEXT NOT NULL REFERENCES "t_team" ("f_id") ON DELETE RESTRICT,

    "f_invitee_qid" TEXT NOT NULL,
    "f_code" TEXT NOT NULL,

    "f_pending" BOOLEAN NOT NULL DEFAULT TRUE,

    "f_role_mask" BIGINT NOT NULL DEFAULT 0,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
