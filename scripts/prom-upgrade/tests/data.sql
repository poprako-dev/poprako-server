INSERT INTO t_local_message
    (f_id, f_topic, f_status, f_payload, f_last_error, f_retried_count, f_lease, f_visible_at, f_created_at, f_updated_at)
VALUES
    ('chapter', 'advance_raw_provide', 'local_message_status:pending',
        '{"AdvanceRawProvide":{"chapter_id":"chapter-1","actor_user_id":"user-1"}}',
        'waiting', 2, 9, '2030-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2025-01-01T00:00:00Z'),
    ('member', 'purge_expired_invitation', 'local_message_status:pending',
        '{"PurgeExpiredInvitation":{"Member":{"invitation_id":"member-1"}}}',
        NULL, 0, 0, '2030-01-01T00:00:00Z', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'),
    ('assignment', 'purge_expired_invitation', 'local_message_status:pending',
        '{"PurgeExpiredInvitation":{"Assignment":{"invitation_id":"assignment-1"}}}',
        NULL, 1, 4, '2030-01-01T00:00:00Z', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'),
    ('dead', 'advance_raw_provide', 'local_message_status:dead', '{}', 'operator diagnosis',
        3, 4, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'),
    ('completed', 'purge_expired_invitation', 'local_message_status:completed', '{}', NULL,
        0, 2, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z');

INSERT INTO t_obj_prom_task
    (f_id, f_topic, f_oper, f_obj_id, f_version, f_key, f_generation, f_status,
     f_visible_at, f_retried_count, f_lease, f_error, f_created_at, f_updated_at)
VALUES
    ('object-pending', 'page_image', 'check', 'page-1', 2, 'key-1', 3, 'obj_prom_status:pending',
        '2030-01-01T00:00:00Z', 2, 4, 'waiting', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'),
    ('object-operator', 'page_image', 'delete', 'page-2', 4, 'key-2', 0, 'obj_prom_status:operator',
        '2025-01-01T00:00:00Z', 3, 7, 'operator diagnosis', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'),
    ('object-completed', 'page_image', 'check', 'page-3', 1, 'key-3', 0, 'obj_prom_status:completed',
        '2025-01-01T00:00:00Z', 0, 1, NULL, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z');
