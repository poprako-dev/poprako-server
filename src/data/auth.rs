pub struct RegisterData {
    pub qid: String,
    pub nickname: String,
    pub password: String,
    pub invitation_code: String,
}

pub struct RegisterVal {
    pub user_id: String,
    pub token: String,
}

pub struct LoginData {
    pub qid: String,
    pub password: String,
}

pub struct LoginVal {
    pub user_id: String,
    pub token: String,
}
