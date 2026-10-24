-- Add editor versions without changing existing review contents.
BEGIN;

ALTER TABLE "t_chapter"
    ADD COLUMN IF NOT EXISTS "f_review_generation" BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS "f_review_revision" BIGINT NOT NULL DEFAULT 0;

ALTER TABLE "t_page_artwork"
    ADD COLUMN IF NOT EXISTS "f_issue_revision" BIGINT NOT NULL DEFAULT 0;

COMMIT;
