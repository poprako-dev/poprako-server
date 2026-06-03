INSERT INTO "t_user" (
    "f_id",
    "f_nickname",
    "f_qid",
    "f_is_sadmin",
    "f_password_hash"
) VALUES (
    'user-00000000-0000-0000-0000-000000000001',
    'SuperAdmin-OvO',
    '123456',
    TRUE,
    '$2a$10$eEEkAsc7h3jdkOyjahdH6OX20w/dHKdGVaH7MNREkh54O57v.E2y2' -- 123456
);

INSERT INTO "t_member" (
    "f_id",
    "f_user_id",
    "f_user_nickname",
    "f_team_id",
    "f_assigned_raw_provider_at",
    "f_assigned_translator_at",
    "f_assigned_proofreader_at",
    "f_assigned_typesetter_at",
    "f_assigned_redrawer_at",
    "f_assigned_reviewer_at",
    "f_assigned_publisher_at",
    "f_assigned_admin_at",
    "f_assigned_assistant_at"
) VALUES (
    'member-00000000-0000-0000-0000-000000000001',
    'user-00000000-0000-0000-0000-000000000001',
    'SuperAdmin-OvO',
    'team-00000000-0000-0000-0000-000000000001',
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW()
);
