use time::OffsetDateTime;

pub struct UserToken {
    pub user_id: String,
}

pub struct UserInfo {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,

    pub is_sadmin: bool,

    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct UserForm {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,
}

pub struct UserInfoUpd<'a> {
    pub id: &'a str,

    pub qid: &'a str,
    pub nickname: &'a str,
}
