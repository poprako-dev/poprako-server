// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use time::OffsetDateTime;
// use uuid::Uuid;
// 
// #[cfg_attr(test, derive(Clone))]
// pub struct SystemMailAggr {
//     pub id: String,
// 
//     pub receiver_id: String,
//     pub read: bool,
// 
//     pub title: String,
//     pub content: String,
// 
//     pub created_at: OffsetDateTime,
// }
// 
// impl SystemMailAggr {
//     /// Returns a new system mail ID with the `sys_mail-` prefix.
//     pub fn generate_id() -> String {
//         format!("sys_mail-{}", Uuid::now_v7())
//     }
// }
// 
// #[derive(Debug)]
// pub struct SystemMailForm {
//     pub id: String,
// 
//     pub receiver_id: String,
// 
//     pub title: String,
//     pub content: String,
// }
