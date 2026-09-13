\set ON_ERROR_STOP on
\pset pager off
\ir contract.sql
BEGIN READ ONLY;
SET LOCAL search_path = public, pg_temp;
SET LOCAL statement_timeout = '30s';

SELECT current_database() AS database, current_setting('server_version') AS postgres_version;
SELECT table_name, column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name IN ('t_local_message', 't_obj_prom_task')
  AND column_name IN ('f_lease', 'f_claim_token') ORDER BY table_name, column_name;

SELECT 'local' AS queue, f_status, count(*) FROM t_local_message GROUP BY f_status
UNION ALL
SELECT 'object', f_status, count(*) FROM t_obj_prom_task GROUP BY f_status ORDER BY 1, 2;

-- Every interrupted invocation requires a decision based on its actual business result.
SELECT 'local' AS queue, f_id, f_topic, f_updated_at
FROM t_local_message WHERE f_status = 'local_message_status:processing'
UNION ALL
SELECT 'object', f_id, f_topic, f_updated_at
FROM t_obj_prom_task WHERE f_status = 'obj_prom_status:processing' ORDER BY 1, 2;

-- Missing actor_user_id is not inferred from chapter ownership.
SELECT f_id, f_topic, f_payload
FROM t_local_message
WHERE f_status = 'local_message_status:pending'
  AND pg_temp.prom_payload(f_payload, f_topic) IS NULL ORDER BY f_id;

-- These requests retain their original age under the new one-hour waiting policy.
SELECT f_id, f_created_at, f_visible_at
FROM t_local_message
WHERE f_status = 'local_message_status:pending'
  AND f_topic IN ('advance_raw_provide', 'chapter')
  AND f_created_at <= now() - interval '1 hour' ORDER BY f_created_at, f_id;

SELECT indexname, indexdef FROM pg_indexes
WHERE schemaname = 'public' AND tablename IN ('t_local_message', 't_obj_prom_task') ORDER BY indexname;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.t_local_message WHERE f_status = 'local_message_status:processing')
        OR EXISTS (SELECT 1 FROM public.t_obj_prom_task WHERE f_status = 'obj_prom_status:processing') THEN
        RAISE EXCEPTION 'Unresolved processing tasks exist; inspect business outcomes before applying';
    END IF;

    IF EXISTS (SELECT 1 FROM public.t_local_message
        WHERE f_status = 'local_message_status:pending' AND pg_temp.prom_payload(f_payload, f_topic) IS NULL) THEN
        RAISE EXCEPTION 'Pending payloads cannot be converted without an explicit data repair';
    END IF;
END
$$;
ROLLBACK;
