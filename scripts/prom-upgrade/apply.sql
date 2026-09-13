\set ON_ERROR_STOP on
\pset pager off
-- Run only while all business servers, producers and consumers are stopped.
BEGIN;
SET LOCAL search_path = public, pg_temp;
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '5min';
DO $$
BEGIN
    IF current_database() NOT IN ('db_poprako_server_prod', 'db_poprako_ci') THEN
        RAISE EXCEPTION 'Expected db_poprako_server_prod or disposable db_poprako_ci';
    END IF;
END
$$;
LOCK TABLE t_local_message, t_obj_prom_task IN ACCESS EXCLUSIVE MODE;
\ir contract.sql

DO $$
DECLARE
    lease_columns INTEGER;
    token_columns INTEGER;
    local_before BIGINT;
    object_before BIGINT;
    local_completed BIGINT;
    object_completed BIGINT;
BEGIN
    SELECT count(*) INTO lease_columns FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name IN ('t_local_message', 't_obj_prom_task')
      AND column_name = 'f_lease' AND data_type = 'bigint' AND is_nullable = 'NO';

    SELECT count(*) INTO token_columns FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name IN ('t_local_message', 't_obj_prom_task')
      AND column_name = 'f_claim_token';

    IF lease_columns = 0 AND token_columns = 2 THEN
        PERFORM pg_temp.assert_prom_target();
        RAISE NOTICE 'Prom upgrade already applied; no queue records changed';
        RETURN;
    END IF;

    IF lease_columns <> 2 OR token_columns <> 0 THEN
        RAISE EXCEPTION 'Expected both original bigint leases or both target UUID tokens; mixed schemas are unsupported';
    END IF;

    IF EXISTS (SELECT 1 FROM t_local_message WHERE f_status = 'local_message_status:processing')
        OR EXISTS (SELECT 1 FROM t_obj_prom_task WHERE f_status = 'obj_prom_status:processing') THEN
        RAISE EXCEPTION 'Unresolved processing tasks exist; migration never automatically replays them';
    END IF;

    IF EXISTS (SELECT 1 FROM t_local_message
        WHERE f_status NOT IN ('local_message_status:pending', 'local_message_status:completed', 'local_message_status:dead'))
        OR EXISTS (SELECT 1 FROM t_obj_prom_task
            WHERE f_status NOT IN ('obj_prom_status:pending', 'obj_prom_status:completed', 'obj_prom_status:operator')) THEN
        RAISE EXCEPTION 'Unknown queue statuses require explicit review';
    END IF;

    IF EXISTS (SELECT 1 FROM t_local_message
        WHERE f_status = 'local_message_status:pending' AND pg_temp.prom_payload(f_payload, f_topic) IS NULL) THEN
        RAISE EXCEPTION 'Invalid pending payload or missing actor_user_id; run preflight.sql';
    END IF;

    SELECT count(*), count(*) FILTER (WHERE f_status = 'local_message_status:completed')
    INTO local_before, local_completed FROM t_local_message;
    SELECT count(*), count(*) FILTER (WHERE f_status = 'obj_prom_status:completed')
    INTO object_before, object_completed FROM t_obj_prom_task;

    UPDATE t_local_message
    SET f_payload = pg_temp.prom_payload(f_payload, f_topic),
        f_topic = CASE WHEN f_topic IN ('advance_raw_provide', 'chapter') THEN 'chapter' ELSE 'invitation' END
    WHERE f_status = 'local_message_status:pending';

    -- Successfully acknowledged history is removed; failed/operator records are retained.
    DELETE FROM t_local_message WHERE f_status = 'local_message_status:completed';
    DELETE FROM t_obj_prom_task WHERE f_status = 'obj_prom_status:completed';

    DROP INDEX IF EXISTS idx_local_message_pending_topic_visible_created;
    DROP INDEX IF EXISTS idx_local_message_processing_updated_lease;
    DROP INDEX IF EXISTS idx_local_message_processing_topic;
    DROP INDEX IF EXISTS idx_local_message_dead_updated;
    DROP INDEX IF EXISTS idx_local_message_completed_updated;
    DROP INDEX IF EXISTS uidx_local_message_processing_topic;
    DROP INDEX IF EXISTS uidx_local_message_pending_chapter;
    DROP INDEX IF EXISTS i_obj_prom_task_stuck;

    ALTER TABLE t_local_message ADD COLUMN f_claim_token UUID;
    ALTER TABLE t_local_message DROP COLUMN f_lease;
    ALTER TABLE t_local_message ADD CONSTRAINT ck_local_message_claim_token CHECK (
        (f_status = 'local_message_status:processing') = (f_claim_token IS NOT NULL));

    ALTER TABLE t_obj_prom_task ADD COLUMN f_claim_token UUID;
    ALTER TABLE t_obj_prom_task DROP COLUMN f_lease;
    ALTER TABLE t_obj_prom_task ADD CONSTRAINT ck_obj_prom_task_claim_token CHECK (
        (f_status = 'obj_prom_status:processing') = (f_claim_token IS NOT NULL));

    CREATE INDEX IF NOT EXISTS idx_local_message_status_topic_visible_created
        ON t_local_message (f_status, f_topic, f_visible_at, f_created_at, f_id);
    CREATE INDEX IF NOT EXISTS idx_local_message_status_updated ON t_local_message (f_status, f_updated_at);
    CREATE INDEX i_obj_prom_task_stuck ON t_obj_prom_task (f_status, f_updated_at);

    PERFORM pg_temp.assert_prom_target();

    IF (SELECT count(*) FROM t_local_message) <> local_before - local_completed
        OR (SELECT count(*) FROM t_obj_prom_task) <> object_before - object_completed THEN
        RAISE EXCEPTION 'Unexpected task loss; rolling back the entire upgrade';
    END IF;

    RAISE NOTICE 'Removed % completed local messages and % completed object tasks; all other tasks retained',
        local_completed, object_completed;
END
$$;
COMMIT;
