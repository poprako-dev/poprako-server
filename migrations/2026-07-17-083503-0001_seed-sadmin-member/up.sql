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
    "f_assigned_bot_at"
) VALUES (
    'member-11111111111',
    'user-11111111111',
    'SuperAdmin-OvO',
    'team-11111111111',
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW(),
    NOW()
)
ON CONFLICT ("f_id") DO NOTHING;
