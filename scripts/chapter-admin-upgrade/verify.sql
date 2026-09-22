\set ON_ERROR_STOP on
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 't_assignment'
            AND column_name = 'f_assigned_admin_at'
    ) OR EXISTS (
        SELECT 1 FROM pg_indexes WHERE schemaname = 'public'
            AND indexname IN ('idx_assignment_chapter_admin_created_at', 'idx_assignment_user_admin_created_at')
    ) THEN
        RAISE EXCEPTION 'Obsolete assignment administrator column or indexes remain';
    END IF;

    IF to_regclass('public.t_assignment') IS NULL THEN
        RAISE EXCEPTION 'Assignment table is missing';
    END IF;
END $$;
