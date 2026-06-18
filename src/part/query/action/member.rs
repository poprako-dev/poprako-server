use poprako_transactional::step::Step;

pub struct MemberUpdUserNickname<'a> {
    pub user_id: &'a str,

    pub user_nickname: &'a str,
}

impl<'a> Step for MemberUpdUserNickname<'a> {
    type Output = ();
}
