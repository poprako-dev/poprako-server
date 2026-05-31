// use time::OffsetDateTime;
//
// use crate::domain::model::value::role::{RoleAssign, RoleView};
//
// use super::role::{RoleFlag, RoleMask};
//
// /// A set of roles with timestamps indicating when each role was assigned.
// pub struct Post {
//     pub assigned_raw_provider_at: Option<OffsetDateTime>,
//     pub assigned_translator_at: Option<OffsetDateTime>,
//     pub assigned_proofreader_at: Option<OffsetDateTime>,
//     pub assigned_typesetter_at: Option<OffsetDateTime>,
//     pub assigned_redrawer_at: Option<OffsetDateTime>,
//     pub assigned_reviewer_at: Option<OffsetDateTime>,
//     pub assigned_publisher_at: Option<OffsetDateTime>,
//     pub assigned_admin_at: Option<OffsetDateTime>,
// }
//
// impl RoleView for Post {
//     fn role_mask(&self) -> RoleMask {
//         let mut mask: u32 = 0;
//
//         if self.assigned_raw_provider_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::RawProvider);
//         }
//         if self.assigned_translator_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Translator);
//         }
//         if self.assigned_proofreader_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Proofreader);
//         }
//         if self.assigned_typesetter_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Typesetter);
//         }
//         if self.assigned_redrawer_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Redrawer);
//         }
//         if self.assigned_reviewer_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Reviewer);
//         }
//         if self.assigned_publisher_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Publisher);
//         }
//         if self.assigned_admin_at.is_some() {
//             mask |= Into::<u32>::into(RoleFlag::Admin);
//         }
//
//         mask.into()
//     }
// }
//
// impl RoleAssign for Post {
//     fn assign_roles(&mut self, mask: RoleMask) {
//         self.assigned_raw_provider_at = if mask.has_role(RoleFlag::RawProvider) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_translator_at = if mask.has_role(RoleFlag::Translator) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_proofreader_at = if mask.has_role(RoleFlag::Proofreader) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_typesetter_at = if mask.has_role(RoleFlag::Typesetter) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_redrawer_at = if mask.has_role(RoleFlag::Redrawer) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_reviewer_at = if mask.has_role(RoleFlag::Reviewer) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_publisher_at = if mask.has_role(RoleFlag::Publisher) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//         self.assigned_admin_at = if mask.has_role(RoleFlag::Admin) {
//             Some(OffsetDateTime::now_utc())
//         } else {
//             None
//         };
//     }
// }
