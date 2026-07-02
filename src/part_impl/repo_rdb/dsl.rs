//! Global DSL module — per-table submodules so consumers write `use dsl::t_xxx::*;`
//! for columns and `use dsl::*;` for table modules.
//!
//! NOTE: `use dsl::*` is the Diesel impl layer exception to rust-use-style.

pub mod t_announcement { pub use super::super::schema::t_announcement::dsl::*; }
pub mod t_assignment { pub use super::super::schema::t_assignment::dsl::*; }
pub mod t_assignment_invitation { pub use super::super::schema::t_assignment_invitation::dsl::*; }
pub mod t_chapter { pub use super::super::schema::t_chapter::dsl::*; }
pub mod t_comic { pub use super::super::schema::t_comic::dsl::*; }
pub mod t_comment { pub use super::super::schema::t_comment::dsl::*; }
pub mod t_local_message { pub use super::super::schema::t_local_message::dsl::*; }
pub mod t_member { pub use super::super::schema::t_member::dsl::*; }
pub mod t_member_invitation { pub use super::super::schema::t_member_invitation::dsl::*; }
pub mod t_page { pub use super::super::schema::t_page::dsl::*; }
pub mod t_system_mail { pub use super::super::schema::t_system_mail::dsl::*; }
pub mod t_team { pub use super::super::schema::t_team::dsl::*; }
pub mod t_unit { pub use super::super::schema::t_unit::dsl::*; }
pub mod t_user { pub use super::super::schema::t_user::dsl::*; }
pub mod t_workset { pub use super::super::schema::t_workset::dsl::*; }
