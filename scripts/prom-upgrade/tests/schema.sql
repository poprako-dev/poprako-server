-- Compare structural contracts, not physical column order.
SELECT table_name, column_name, udt_name, is_nullable, column_default
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name IN ('t_local_message', 't_obj_prom_task')
ORDER BY table_name, column_name;
SELECT conrelid::regclass, conname, pg_get_constraintdef(oid), convalidated
FROM pg_constraint WHERE conrelid IN ('t_local_message'::regclass, 't_obj_prom_task'::regclass)
ORDER BY conrelid::regclass::text, conname;
SELECT tablename, indexname, indexdef FROM pg_indexes
WHERE schemaname = 'public' AND tablename IN ('t_local_message', 't_obj_prom_task') ORDER BY tablename, indexname;
