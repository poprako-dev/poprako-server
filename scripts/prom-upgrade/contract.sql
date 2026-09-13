-- Session-local helpers; no migration functions remain in the application schema.
CREATE OR REPLACE FUNCTION pg_temp.prom_payload(payload JSONB, topic TEXT)
RETURNS JSONB LANGUAGE plpgsql AS $$
DECLARE
    converted JSONB;
    operation JSONB;
BEGIN
    IF topic = 'advance_raw_provide'
        AND payload = jsonb_build_object('AdvanceRawProvide', payload->'AdvanceRawProvide') THEN
        converted := jsonb_build_object('Chapter', jsonb_build_object(
            'payload', jsonb_build_object('TryAdvanceRawProvideStage', payload->'AdvanceRawProvide')));
    ELSIF topic = 'purge_expired_invitation'
        AND payload = jsonb_build_object('PurgeExpiredInvitation',
            jsonb_build_object('Member', payload#>'{PurgeExpiredInvitation,Member}')) THEN
        converted := jsonb_build_object('Invitation', jsonb_build_object(
            'payload', jsonb_build_object('PurgeExpiredMemberInvitation', payload#>'{PurgeExpiredInvitation,Member}')));
    ELSIF topic = 'purge_expired_invitation'
        AND payload = jsonb_build_object('PurgeExpiredInvitation',
            jsonb_build_object('Assignment', payload#>'{PurgeExpiredInvitation,Assignment}')) THEN
        converted := jsonb_build_object('Invitation', jsonb_build_object(
            'payload', jsonb_build_object('PurgeExpiredAssignmentInvitation', payload#>'{PurgeExpiredInvitation,Assignment}')));
    ELSIF topic IN ('chapter', 'invitation') THEN
        converted := payload;
    ELSE
        RETURN NULL;
    END IF;

    IF topic IN ('advance_raw_provide', 'chapter') THEN
        operation := converted#>'{Chapter,payload,TryAdvanceRawProvideStage}';

        IF jsonb_typeof(operation->'chapter_id') = 'string'
            AND length(btrim(operation->>'chapter_id')) > 0
            AND jsonb_typeof(operation->'actor_user_id') = 'string'
            AND length(btrim(operation->>'actor_user_id')) > 0
            AND converted = jsonb_build_object('Chapter', jsonb_build_object(
                'payload', jsonb_build_object('TryAdvanceRawProvideStage', operation))) THEN
            RETURN converted;
        END IF;

        RETURN NULL;
    END IF;

    FOREACH topic IN ARRAY ARRAY['PurgeExpiredMemberInvitation', 'PurgeExpiredAssignmentInvitation'] LOOP
        operation := converted#>ARRAY['Invitation', 'payload', topic];

        IF jsonb_typeof(operation->'invitation_id') = 'string'
            AND length(btrim(operation->>'invitation_id')) > 0
            AND converted = jsonb_build_object('Invitation', jsonb_build_object(
                'payload', jsonb_build_object(topic, operation))) THEN
            RETURN converted;
        END IF;
    END LOOP;

    RETURN NULL;
END
$$;

CREATE OR REPLACE FUNCTION pg_temp.assert_prom_target()
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    IF (SELECT count(*) FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name IN ('t_local_message', 't_obj_prom_task')
          AND column_name = 'f_claim_token' AND data_type = 'uuid'
          AND is_nullable = 'YES' AND column_default IS NULL) <> 2
        OR EXISTS (SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name IN ('t_local_message', 't_obj_prom_task')
              AND column_name = 'f_lease') THEN
        RAISE EXCEPTION 'Prom columns do not match the UUID claim-token schema';
    END IF;

    IF (SELECT count(*) FROM pg_constraint
        WHERE convalidated AND contype = 'c'
          AND ((conrelid = 'public.t_local_message'::regclass AND conname = 'ck_local_message_claim_token')
            OR (conrelid = 'public.t_obj_prom_task'::regclass AND conname = 'ck_obj_prom_task_claim_token'))
          AND pg_get_constraintdef(oid) = format(
              'CHECK (((f_status = %L::text) = (f_claim_token IS NOT NULL)))',
              CASE conrelid WHEN 'public.t_local_message'::regclass
                  THEN 'local_message_status:processing' ELSE 'obj_prom_status:processing' END)) <> 2 THEN
        RAISE EXCEPTION 'Prom claim-token constraints are missing, unvalidated or incorrect';
    END IF;

    IF EXISTS (SELECT 1 FROM public.t_local_message
        WHERE f_status NOT IN ('local_message_status:pending', 'local_message_status:processing', 'local_message_status:dead')
          OR ((f_status = 'local_message_status:processing') <> (f_claim_token IS NOT NULL))
          OR (f_status <> 'local_message_status:dead'
              AND (f_topic NOT IN ('chapter', 'invitation')
                  OR pg_temp.prom_payload(f_payload, f_topic) IS DISTINCT FROM f_payload))) THEN
        RAISE EXCEPTION 'Local messages contain unsupported active payloads, topics, statuses or tokens';
    END IF;

    IF EXISTS (SELECT 1 FROM public.t_obj_prom_task
        WHERE f_status NOT IN ('obj_prom_status:pending', 'obj_prom_status:processing', 'obj_prom_status:operator')
          OR ((f_status = 'obj_prom_status:processing') <> (f_claim_token IS NOT NULL))) THEN
        RAISE EXCEPTION 'Object tasks contain unsupported statuses or tokens';
    END IF;

    IF (SELECT count(*) FROM (VALUES
        ('t_local_message', 'idx_local_message_status_topic_visible_created', ARRAY['f_status', 'f_topic', 'f_visible_at', 'f_created_at', 'f_id']),
        ('t_local_message', 'idx_local_message_status_updated', ARRAY['f_status', 'f_updated_at']),
        ('t_obj_prom_task', 'i_obj_prom_task_poll', ARRAY['f_status', 'f_visible_at', 'f_created_at', 'f_id']),
        ('t_obj_prom_task', 'i_obj_prom_task_stuck', ARRAY['f_status', 'f_updated_at'])
    ) AS expected(table_name, index_name, columns)
        JOIN pg_index i ON i.indexrelid = to_regclass('public.' || expected.index_name)
        JOIN pg_class c ON c.oid = i.indexrelid
        JOIN pg_am am ON am.oid = c.relam
        WHERE i.indrelid = to_regclass('public.' || expected.table_name)
          AND i.indisvalid AND NOT i.indisunique AND i.indpred IS NULL
          AND am.amname = 'btree' AND i.indnatts = cardinality(expected.columns)
          AND ARRAY(SELECT pg_get_indexdef(i.indexrelid, n, true)
              FROM generate_series(1, i.indnatts) n ORDER BY n) = expected.columns) <> 4 THEN
        RAISE EXCEPTION 'Prom indexes do not match the four ordinary target indexes';
    END IF;

    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = 'public' AND indexname IN (
        'idx_local_message_pending_topic_visible_created', 'idx_local_message_processing_updated_lease',
        'idx_local_message_processing_topic', 'idx_local_message_dead_updated', 'idx_local_message_completed_updated',
        'uidx_local_message_processing_topic', 'uidx_local_message_pending_chapter')) THEN
        RAISE EXCEPTION 'Superseded local-message indexes remain';
    END IF;

    IF (SELECT count(*) FROM pg_index
        WHERE indrelid IN ('public.t_local_message'::regclass, 'public.t_obj_prom_task'::regclass)
          AND NOT indisprimary) <> 4 THEN
        RAISE EXCEPTION 'Unexpected secondary queue indexes require review';
    END IF;

END
$$;
