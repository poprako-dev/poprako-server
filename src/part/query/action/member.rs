use poprako_transactional::step::Step;

pub struct UpdateUserNickname<'a> {
    pub user_id: &'a str,

    pub user_nickname: &'a str,
}

impl<'a> Step for UpdateUserNickname<'a> {
    type Output = ();
}
