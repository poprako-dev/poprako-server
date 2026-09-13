\set ON_ERROR_STOP on
\pset pager off
\ir contract.sql
BEGIN READ ONLY;
SET LOCAL search_path = public, pg_temp;
SET LOCAL statement_timeout = '30s';
SELECT pg_temp.assert_prom_target();
SELECT 'local' AS queue, f_status, count(*) FROM t_local_message GROUP BY f_status
UNION ALL
SELECT 'object', f_status, count(*) FROM t_obj_prom_task GROUP BY f_status ORDER BY 1, 2;
SELECT table_name, column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name IN ('t_local_message', 't_obj_prom_task')
  AND column_name IN ('f_lease', 'f_claim_token') ORDER BY table_name, column_name;
SELECT conrelid::regclass AS table_name, conname, pg_get_constraintdef(oid)
FROM pg_constraint
WHERE conname IN ('ck_local_message_claim_token', 'ck_obj_prom_task_claim_token');
SELECT indexname, indexdef FROM pg_indexes
WHERE schemaname = 'public' AND tablename IN ('t_local_message', 't_obj_prom_task') ORDER BY indexname;
ROLLBACK;
