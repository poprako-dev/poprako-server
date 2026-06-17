use poprako_transactional::step::Step;

// use crate::model::user::UserForm;
//
// pub struct Create<'a> {
//     pub form: &'a UserForm,
// }
//
// impl<'a> Step for Create<'a> {
//     type Output = ();
// }
//
// pub struct TouchLastActive<'a> {
//     pub id: &'a str,
// }
//
// impl<'a> Step for TouchLastActive<'a> {
//     type Output = ();
// }
//

pub struct UpdateInfo<'a> {
    pub id: &'a str,

    pub qid: &'a str,
    pub nickname: &'a str,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}
