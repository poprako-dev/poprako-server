DO $$
BEGIN
    IF (SELECT count(*) FROM t_local_message) <> 4 OR (SELECT count(*) FROM t_obj_prom_task) <> 2 THEN
        RAISE EXCEPTION 'Only completed records should be removed';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM t_local_message WHERE f_id = 'chapter'
        AND f_topic = 'chapter' AND f_status = 'local_message_status:pending'
        AND f_payload = '{"Chapter":{"payload":{"TryAdvanceRawProvideStage":{"chapter_id":"chapter-1","actor_user_id":"user-1"}}}}'::jsonb
        AND f_created_at = '2020-01-01T00:00:00Z' AND f_visible_at = '2030-01-01T00:00:00Z'
        AND f_updated_at = '2025-01-01T00:00:00Z' AND f_retried_count = 2
        AND f_last_error = 'waiting' AND f_claim_token IS NULL) THEN
        RAISE EXCEPTION 'Chapter request fields were lost or its waiting age was reset';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM t_local_message WHERE f_id = 'member' AND f_topic = 'invitation'
        AND f_payload = '{"Invitation":{"payload":{"PurgeExpiredMemberInvitation":{"invitation_id":"member-1"}}}}'::jsonb
        AND f_visible_at = '2030-01-01T00:00:00Z' AND f_retried_count = 0)
        OR NOT EXISTS (SELECT 1 FROM t_local_message WHERE f_id = 'assignment' AND f_topic = 'invitation'
        AND f_payload = '{"Invitation":{"payload":{"PurgeExpiredAssignmentInvitation":{"invitation_id":"assignment-1"}}}}'::jsonb
        AND f_visible_at = '2030-01-01T00:00:00Z' AND f_retried_count = 1) THEN
        RAISE EXCEPTION 'Delayed invitation requests were not preserved';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM t_local_message WHERE f_id = 'dead'
        AND f_status = 'local_message_status:dead' AND f_payload = '{}'::jsonb
        AND f_last_error = 'operator diagnosis' AND f_retried_count = 3)
        OR NOT EXISTS (SELECT 1 FROM t_obj_prom_task WHERE f_id = 'object-operator'
        AND f_status = 'obj_prom_status:operator' AND f_error = 'operator diagnosis' AND f_retried_count = 3) THEN
        RAISE EXCEPTION 'Failed tasks were overwritten or requeued';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM t_obj_prom_task WHERE f_id = 'object-pending'
        AND f_topic = 'page_image' AND f_oper = 'check' AND f_obj_id = 'page-1'
        AND f_version = 2 AND f_key = 'key-1' AND f_generation = 3
        AND f_status = 'obj_prom_status:pending' AND f_retried_count = 2
        AND f_visible_at = '2030-01-01T00:00:00Z' AND f_error = 'waiting'
        AND f_created_at = '2025-01-01T00:00:00Z' AND f_updated_at = '2025-01-01T00:00:00Z'
        AND f_claim_token IS NULL) THEN
        RAISE EXCEPTION 'Object obligation identity or lifecycle data changed';
    END IF;
END
$$;
