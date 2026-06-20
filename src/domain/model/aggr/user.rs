// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use std::mem;
// 
// use time::OffsetDateTime;
// use uuid::Uuid;
// 
// use crate::domain::model::event::{Event, EventEmit, EventSink};
// 
// #[cfg_attr(test, derive(Clone))]
// pub struct UserAggr {
//     pub id: String,
// 
//     pub qid: String,
//     pub nickname: String,
// 
//     pub avatar_key: Option<String>,
//     pub avatar_uploaded: bool,
//     pub avatar_version: i64,
// 
//     pub is_sadmin: bool,
// 
//     pub last_active_at: OffsetDateTime,
// 
//     pub created_at: OffsetDateTime,
//     pub updated_at: OffsetDateTime,
// }
// 
// impl UserAggr {
//     pub fn generate_id() -> String {
//         format!("user-{}", Uuid::now_v7())
//     }
// 
//     pub fn generate_avatar_key(user_id: &str, avatar_version: i64, ext: &str) -> String {
//         format!("user_avatar/{}-{}.{}", user_id, avatar_version, ext)
//     }
// }
// 
// pub struct UserAvatarReservation {
//     pub object_key: String,
//     pub previous_object_key: Option<String>,
//     pub avatar_version: i64,
// }
// 
// #[derive(Debug, Clone)]
// pub struct UserToken {
//     pub user_id: String,
// }
// 
// #[cfg_attr(test, derive(Clone))]
// pub struct UserCredential {
//     pub user_id: String,
//     pub password_hash: String,
// }
// 
// impl UserCredential {
//     pub fn verify_password(&self, password: &str) -> bool {
//         bcrypt::verify(password, &self.password_hash).unwrap_or(false)
//     }
// }
// 
// pub struct UserForm {
//     pub id: String,
// 
//     pub qid: String,
//     pub nickname: String,
// 
//     pub password_hash: String,
// 
//     events: Vec<Event>,
// }
// 
// impl UserForm {
//     pub fn new(id: String, qid: String, nickname: String, password_hash: String) -> Self {
//         Self {
//             id,
//             qid,
//             nickname,
//             password_hash,
//             events: Vec::new(),
//         }
//     }
// }
// 
// impl EventSink for UserForm {
//     fn push_event(&mut self, event: Event) {
//         self.events.push(event);
//     }
// }
// 
// impl EventEmit for UserForm {
//     fn pull_events(&mut self) -> Vec<Event> {
//         // A swap-and-clear pattern to avoid cloning the events.
//         mem::take(&mut self.events)
//     }
// }
// 
// /// Input aggregate for updating user profile fields via PUT.
// ///
// /// Does NOT include password — password updates use a separate flow.
// pub struct UserInfoUpdate {
//     pub id: String,
// 
//     pub qid: String,
//     pub nickname: String,
// }
// 
// #[cfg(test)]
// mod tests {
//     // verify_password_match(UserCredential::verify_password)(positive): password verification should pass for the original password.
//     // verify_password_mismatch(UserCredential::verify_password)(negative): password verification should fail for a wrong password.
//     // verify_password_corrupted_hash_returns_false(UserCredential::verify_password)(negative): corrupted hashes should fail instead of panicking.
//     // clone_without_events_preserves_fields_clears_events(UserForm::clone_without_events)(positive): cloning should preserve fields and clear cloned events.
//     // push_and_pull_events_swap_and_clear(UserForm::push_event/UserForm::pull_events)(positive): pulling events should return all pending events and clear the buffer.
// 
//     use super::*;
// 
//     use crate::domain::model::aggr::user::UserAggr;
//     use crate::domain::model::event::user::UserSignedUpEvent;
//     use crate::domain::model::event::{Event, EventEmit, EventSink};
// 
//     #[test]
//     fn verify_password_match() {
//         let hash = bcrypt::hash("secret123", bcrypt::DEFAULT_COST).unwrap();
//         let credential = UserCredential {
//             user_id: "user-1".into(),
//             password_hash: hash,
//         };
//         assert!(credential.verify_password("secret123"));
//     }
// 
//     #[test]
//     fn verify_password_mismatch() {
//         let hash = bcrypt::hash("secret123", bcrypt::DEFAULT_COST).unwrap();
//         let credential = UserCredential {
//             user_id: "user-1".into(),
//             password_hash: hash,
//         };
//         assert!(!credential.verify_password("wrong"));
//     }
// 
//     #[test]
//     fn verify_password_corrupted_hash_returns_false() {
//         let credential = UserCredential {
//             user_id: "user-1".into(),
//             password_hash: "not-a-valid-bcrypt-hash".into(),
//         };
//         assert!(!credential.verify_password("anything"));
//     }
// 
//     #[test]
//     fn push_and_pull_events_swap_and_clear() {
//         let mut form = UserForm::new(
//             UserAggr::generate_id(),
//             "qid".into(),
//             "nick".into(),
//             "pw".into(),
//         );
// 
//         let ev1 = Event::UserSignedUp(UserSignedUpEvent {
//             team_id: "t".into(),
//             invitor_id: "u".into(),
//             invitee_qid: "q".into(),
//         });
//         let ev2 = Event::UserSignedUp(UserSignedUpEvent {
//             team_id: "t2".into(),
//             invitor_id: "u2".into(),
//             invitee_qid: "q2".into(),
//         });
//         form.push_event(ev1);
//         form.push_event(ev2);
// 
//         let pulled = form.pull_events();
//         assert_eq!(pulled.len(), 2);
//         assert!(form.pull_events().is_empty());
//     }
// }
