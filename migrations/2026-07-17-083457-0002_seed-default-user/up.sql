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
