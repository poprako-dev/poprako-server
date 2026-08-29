//! Pure rules for comic entities.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_admin_with_roles,
    check_user_is_team_member,
};
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::index::stored_index_to_user_index;
use crate::value::role::RoleMask;

/// Pure domain operations for comic entities.
pub struct ComicComplex;

impl ComicComplex {
    /// Rejects ordinary mutations after a comic has been archived.
    pub fn ensure_comic_writable(comic_info: &ComicInfo) -> BaseRest<()> {
        //
        if comic_info.archived_at.is_none() {
            return accept(());
        }

        let err_message = trl("error-comic-archived");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %comic_info.id,
            "expected error: archived comic is frozen",
        );

        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        })
    }

    /// Generate a unique, time-ordered comic identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Compose a display title from raw fields for search materialization.
    pub fn compose_title(index: usize, author: &str, title: &str) -> String {
        format!("{} {} {}", stored_index_to_user_index(index), author, title)
    }
}

/// Pure permission rules for comic entities.
pub struct ComicPermComplex;

impl ComicPermComplex {
    /// Verify the caller may create a comic with the requested preset roles.
    pub fn ensure_user_can_create(
        member_info: &MemberInfo,
        preset_assignment_roles: Option<RoleMask>,
    ) -> BaseRest<()> {
        //
        check_user_is_team_admin_with_roles(
            member_info,
            preset_assignment_roles,
        )
    }

    /// Verify the caller may list comics.
    pub const fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may read a comic.
    pub const fn ensure_user_can_get_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may update a comic.
    pub fn ensure_user_can_update_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may reserve a comic cover.
    pub fn ensure_user_can_reserve_cover(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may confirm a comic cover upload.
    pub fn ensure_user_can_mark_cover_uploaded(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may delete a comic.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }
}
