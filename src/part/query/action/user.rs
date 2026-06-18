use poprako_transactional::step::Step;

use crate::model;

pub struct UserGetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserGetInfoById<'a> {
    type Output = model::user::UserInfo;
}

pub struct UserUpdInfo<'a> {
    pub id: &'a str,

    pub qid: &'a str,
    pub nickname: &'a str,
}

impl<'a> Step for UserUpdInfo<'a> {
    type Output = ();
}
