CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_id"
    ON "t_assignment_invitation" ("f_chapter_id");

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_invitee_qid_pending"
    ON "t_assignment_invitation" ("f_invitee_qid", "f_pending", "f_created_at" DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_invitation_pending_code"
    ON "t_assignment_invitation" ("f_code")
    WHERE "f_pending" = TRUE;

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_invitation_pending_chapter_invitee"
    ON "t_assignment_invitation" ("f_chapter_id", "f_invitee_qid")
    WHERE "f_pending" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_created_at"
    ON "t_assignment_invitation" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC);

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_pending_created_at"
    ON "t_assignment_invitation" ("f_chapter_id", "f_pending", "f_created_at" DESC, "f_id" ASC);
