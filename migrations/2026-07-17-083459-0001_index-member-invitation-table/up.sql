CREATE INDEX IF NOT EXISTS "idx_member_invitation_invitee_qid"
    ON "t_member_invitation" ("f_invitee_qid");

CREATE INDEX IF NOT EXISTS "idx_member_invitation_pending_code"
    ON "t_member_invitation" ("f_code")
    WHERE "f_pending" = TRUE;

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_member_invitation_team_id_invitee_qid_pending"
    ON "t_member_invitation" ("f_team_id", "f_invitee_qid")
    WHERE "f_pending" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_member_invitation_team_id_created_at_desc"
    ON "t_member_invitation" ("f_team_id", "f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_member_invitation_team_pending_created_at_desc"
    ON "t_member_invitation" ("f_team_id", "f_pending", "f_created_at" DESC);
