//! Domain models for chapter assignments — role masks tracking which
//! workflow roles a user holds for a chapter.
//!
//! Each assignment carries a [`RoleMask`] bitmap. The individual
//! workflow stages a user can act on are derived from the mask.
//!
//! Convert to [`AssignmentInfoVal`] for presentation.
//!
//! [`RoleMask`]: crate::value::role::RoleMask
//! [`AssignmentInfoVal`]: crate::data::chapter::AssignmentInfoVal

use time::OffsetDateTime;

use crate::model::{chapter_model, user_model};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// A chapter assignment record linking a user to a chapter with a set of
/// workflow roles.
///
/// The `roles` mask specifies which workflow roles the user holds for this
/// chapter (translator, proofreader, etc.).
#[cfg_attr(test, derive(Clone))]
pub struct Info {
    pub id: String,

    pub chapter_id: String,
    pub user_id: String,

    pub user: Option<user_model::Info>,
    pub chapter: Option<chapter_model::Info>,

    pub roles: RoleMask,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert a new assignment row.
///
/// The `roles` mask specifies the initial set of roles.
#[cfg_attr(test, derive(Clone))]
pub struct Form {
    pub id: String,

    pub chapter_id: String,
    pub user_id: String,

    pub roles: RoleMask,
}

/// Mutable role fields for a chapter assignment.
///
/// Carries the merged [`RoleMask`] after adding (or removing) roles.
/// The use case layer computes the merge via [`AssignmentComplex::merge_roles`]
/// before constructing this update.
///
/// [`AssignmentComplex::merge_roles`]: crate::complex::assignment::AssignmentComplex::merge_roles
#[cfg_attr(test, derive(Clone))]
pub struct RoleUpdate {
    pub id: String,

    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing chapter assignments.
pub enum ListSpec {
    Chapter {
        chapter_id: String,
        role: Option<RoleField>,
        incl_opt: Vec<AssignmentInclOpt>,
        offset: u64,
        limit: u64,
    },
    User {
        owner_id: String,
        role: Option<RoleField>,
        incl_opt: Vec<AssignmentInclOpt>,
        offset: u64,
        limit: u64,
    },
}
