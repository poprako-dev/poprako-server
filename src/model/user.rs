pub struct Form {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,
}

pub struct InfoUpdate<'a> {
    pub id: &'a str,

    pub qid: &'a str,
    pub nickname: &'a str,
}
