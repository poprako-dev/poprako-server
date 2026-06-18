use poprako_transactional::step::Step;

use crate::model::member::{MemberForm, MemberInfo};

pub struct MemberCreate<'a> {
    pub form: &'a MemberForm,
}

impl<'a> Step for MemberCreate<'a> {
    type Output = MemberInfo;
}

pub struct MemberUpdateUserNickname<'a> {
    pub user_id: &'a str,
    pub user_nickname: &'a str,
}

impl<'a> Step for MemberUpdateUserNickname<'a> {
    type Output = ();
}

pub struct MemberTouchLastActive<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for MemberTouchLastActive<'a> {
    type Output = ();
}

pub struct MemberListByUserIdExcluded<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for MemberListByUserIdExcluded<'a> {
    type Output = Vec<MemberInfo>;
}

pub struct MemberDelete<'a> {
    pub id: &'a str,
}

impl<'a> Step for MemberDelete<'a> {
    type Output = ();
}
