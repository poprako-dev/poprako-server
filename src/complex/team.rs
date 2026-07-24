//! Complex-domain opers for team entities: identity and avatar-storage key
//! generation, and permission checks.

use poprako_orchestra::Proxy;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::termbase::TermbaseComplex;
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
use crate::part::repo::oper::member::{
    DeleteMember, FindMemberInfo, ListMemberInfosExcluded,
};
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::team::{DeleteTeam, GetTeamInfoExcluded};
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::oper::workset::{
    DeleteWorkset, GetWorksetInfoExcluded, ListWorksetInfosExcluded,
    UpdateWorksetComicCount,
};
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
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
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseResult<()>
    where
        P: for<'a> Proxy<GetTeamInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<ListWorksetInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTeam<'a>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<ListComicInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteWorkset<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteComic<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateWorksetComicCount<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<GetChapterInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListPageInfos<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignmentInvitations<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignments<'a>, Error = BaseError>
            + for<'a> Proxy<DeletePages<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UnpinOtherChapters<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateComicChapterCount<'a>, Error = BaseError>
            + for<'a> Proxy<TouchComicLastActive<'a>, Error = BaseError>
            + for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>
            + for<'a> Proxy<ListMemberInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteMember<'a>, Error = BaseError>
            + for<'a> Proxy<Defer<'a, String, Payload, ()>, Error = BaseError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, Payload, ()>,
                Error = BaseError,
            >,
    {
        // SAFETY: Lock the root team row (FOR UPDATE) to serialize with
        // concurrent workset creations, preventing resource leaks from
        // worksets (and their subtrees) inserted between the listing and
        // the team delete.

        let team_info = proxy.exec(&GetTeamInfoExcluded::Id { id }).await?;

        TermbaseComplex::delete_team_cascade(proxy, &team_info.id).await?;

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

        let member_infos = proxy
            .exec(&ListMemberInfosExcluded::Team {
                team_id: &team_info.id,
            })
            .await?;

        for member_info in member_infos {
            proxy
                .exec(&DeleteMember {
                    id: &member_info.id,
                })
                .await?;
        }

        proxy.exec(&DeleteTeam { id: &team_info.id }).await?;

        accept(())
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
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_reserve_avatar<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_mark_avatar_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the user can create a team.
    pub async fn ensure_user_can_create<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<GetUserInfo<'a>, Error = BaseError>,
    {
        Self::check_user_is_sadmin(proxy, user_id).await
    }

    /// Verify the user can list team infos.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<GetUserInfo<'a>, Error = BaseError>,
    {
        Self::check_user_is_sadmin(proxy, user_id).await
    }

    /// Check whether the user is a super-admin; returns a `Perm` error if not.
    async fn check_user_is_sadmin<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<GetUserInfo<'a>, Error = BaseError>,
    {
        let user_info = proxy.exec(&GetUserInfo::Id { id: user_id }).await?;

        if !user_info.is_sadmin {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-sadmin-required"),
            });
        }

        accept(())
    }
}
