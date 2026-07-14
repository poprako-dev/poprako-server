//! Complex-domain opers for team entities: identity and avatar-storage key
//! generation, and permission checks.

use poprako_orchestra::Proxy;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::util::check_user_is_team_admin;
use crate::complex::workset::WorksetComplex;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::comic::{
    DeleteComic, GetComicInfoExcluded, ListComicInfosExcluded,
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::team::{DeleteTeam, GetTeamInfoExcluded};
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::oper::workset::{
    DeleteWorkset, GetWorksetInfoExcluded, ListWorksetInfosExcluded,
    UpdateWorksetComicCount,
};
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::next_snowflake_id;

/// Domain opers for team entities.
pub struct TeamComplex;

impl TeamComplex {
    /// Generate a unique, time-ordered team identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Generate the object-storage key for a team avatar image.
    pub fn gen_avatar_key(
        id: &str,
        avatar_version: u32,
        file_ext: &str,
    ) -> String {
        format!("team_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }

    /// Deletes a team subtree inside an existing transaction context.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> RegularResult<()>
    where
        P: for<'a> Proxy<GetTeamInfoExcluded<'a>, Error = RegularError>
            + for<'a> Proxy<ListWorksetInfosExcluded<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteTeam<'a>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfoExcluded<'a>, Error = RegularError>
            + for<'a> Proxy<ListComicInfosExcluded<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteWorkset<'a>, Error = RegularError>
            + for<'a, 'b> Proxy<
                GetComicInfoExcluded<'a, 'b>,
                Error = RegularError,
            > + for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteComic<'a>, Error = RegularError>
            + for<'a> Proxy<UpdateWorksetComicCount<'a>, Error = RegularError>
            + for<'a, 'b> Proxy<
                GetChapterInfoExcluded<'a, 'b>,
                Error = RegularError,
            > + for<'a> Proxy<ListPageInfos<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteAssignmentInvitations<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteAssignments<'a>, Error = RegularError>
            + for<'a> Proxy<DeletePages<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteChapter<'a>, Error = RegularError>
            + for<'a> Proxy<UpdateChapter<'a>, Error = RegularError>
            + for<'a> Proxy<UnpinOtherChapters<'a>, Error = RegularError>
            + for<'a> Proxy<UpdateComicChapterCount<'a>, Error = RegularError>
            + for<'a> Proxy<TouchComicLastActive<'a>, Error = RegularError>
            + for<'a> Proxy<Defer<'a, String, Payload, ()>, Error = RegularError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, Payload, ()>,
                Error = RegularError,
            >,
    {
        // SAFETY: Lock the root team row (FOR UPDATE) to serialize with
        // concurrent workset creations, preventing resource leaks from
        // worksets (and their subtrees) inserted between the listing and
        // the team delete.

        let team_info = proxy.exec(&GetTeamInfoExcluded::Id { id }).await?;

        let workset_infos = proxy
            .exec(&ListWorksetInfosExcluded {
                team_id: &team_info.id,
            })
            .await?;

        for workset_info in workset_infos {
            WorksetComplex::delete_cascade(proxy, &workset_info.id).await?;
        }

        if let Some(avatar_key) = &team_info.avatar_key
            && team_info.avatar_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();

            let payload = Payload::Image(image::Payload::Delete {
                object_key: avatar_key.clone(),
            });

            let task = Task {
                id: &delete_id,
                payload: &payload,
                delay: None,
            };

            proxy.exec(&Defer::new(task)).await?;
        }

        proxy.exec(&DeleteTeam { id: &team_info.id }).await?;

        Ok(())
    }
}

/// Permission-gate opers for team entities.
pub struct TeamPermComplex;

impl TeamPermComplex {
    /// Verify the user has super-admin privileges required to list all teams.
    /// Returns an `Expected::Perm` error if the user is not a super-admin.
    pub async fn ensure_user_can_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_reserve_avatar<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_mark_avatar_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the user has super-admin privileges required to list all teams.
    pub async fn ensure_user_can_list_all<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<GetUserInfo<'a>, Error = RegularError>,
    {
        Self::check_user_is_sadmin(proxy, user_id).await
    }

    /// Check whether the user is a super-admin; returns a `Perm` error if not.
    async fn check_user_is_sadmin<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<GetUserInfo<'a>, Error = RegularError>,
    {
        let user_info = proxy.exec(&GetUserInfo::Id { id: user_id }).await?;

        if !user_info.is_sadmin {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-sadmin-required"),
            });
        }

        Ok(())
    }
}
