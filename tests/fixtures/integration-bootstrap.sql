INSERT INTO "t_user" (
    "f_id",
    "f_nickname",
    "f_qid",
    "f_is_sadmin",
    "f_password_hash"
) VALUES (
    'user-11111111111',
    'SuperAdmin-OvO',
    '123456',
    TRUE,
    '$argon2id$v=19$m=65536,t=3,p=4$UrCPl9xY0hk3LpfQWl+ZVA$4d+zkTiD9ghoc6XtJJSHpcvfzUpAK1IiZ5MAQezLgrE'
)
ON CONFLICT ("f_id") DO NOTHING;

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
    NOW(), NOW(), NOW(), NOW(), NOW(), NOW(), NOW(), NOW(), NOW()
)
ON CONFLICT ("f_id") DO NOTHING;
