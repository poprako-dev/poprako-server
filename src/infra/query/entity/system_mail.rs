// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use diesel::prelude::*;
// use time::OffsetDateTime;
// 
// use crate::infra::query::schema;
// 
// // ── Insertable ─────────────────────────────────────────────────────────────
// 
// #[derive(Insertable)]
// #[diesel(table_name = schema::t_system_mail)]
// pub struct SystemMailEntry<'a> {
//     pub f_id: &'a str,
//     pub f_receiver_id: &'a str,
//     pub f_title: &'a str,
//     pub f_content: &'a str,
//     pub f_created_at: OffsetDateTime,
// }
