use poprako_transactional::step::Step;

use crate::model::member::{MemberForm, MemberInfo};

pub struct Create<'a> {
    pub form: &'a MemberForm,
}

impl<'a> Step for Create<'a> {
    type Output = MemberInfo;
}

pub struct UpdateUserNickname<'a> {
    pub user_id: &'a str,
    pub user_nickname: &'a str,
}

impl<'a> Step for UpdateUserNickname<'a> {
    type Output = ();
}

pub struct TouchLastActive<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for TouchLastActive<'a> {
    type Output = ();
}

pub struct ListByUserIdExcluded<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for ListByUserIdExcluded<'a> {
    type Output = Vec<MemberInfo>;
}

pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

pub struct MemberStep;

impl MemberStep {
    pub fn create<'a>(form: &'a MemberForm) -> Create<'a> {
        Create { form }
    }

    pub fn update_user_nickname<'a>(
        user_id: &'a str,
        user_nickname: &'a str,
    ) -> UpdateUserNickname<'a> {
        UpdateUserNickname {
            user_id,
            user_nickname,
        }
    }

    pub fn touch_last_active<'a>(user_id: &'a str) -> TouchLastActive<'a> {
        TouchLastActive { user_id }
    }

    pub fn list_by_user_id_excluded<'a>(user_id: &'a str) -> ListByUserIdExcluded<'a> {
        ListByUserIdExcluded { user_id }
    }

    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
