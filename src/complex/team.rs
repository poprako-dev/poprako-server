//! Complex-domain opers for team entities: identity and avatar-storage key
//! generation, and permission checks.

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::util::check_user_is_team_admin;
use crate::complex::workset::WorksetComplex;
use crate::part::prom::task::{IMAGE_TOPIC, ImageTask};
use crate::part::prom::{Payload, Prom, PromStep};
use crate::part::repo::assignment::AssignmentRepoTransactional;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepoTransactional;
use crate::part::repo::chapter::ChapterRepoTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::{GetInfoById, UserStep};
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::team::TeamRepoTransactional;
use crate::part::repo::unit::UnitRepoTransactional;
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::part::shared::proxy::ProxyExecute;
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
        avatar_version: i64,
        file_ext: &str,
    ) -> String {
        format!("team_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }

    /// Deletes a team subtree inside an existing transaction context.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RegularResult<()>
    where
        C: Send,
        R: TeamRepoTransactional<C>
            + WorksetRepoTransactional<C>
            + ComicRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + PageRepoTransactional<C>
            + AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + UnitRepoTransactional<C>
            + Send
            + Sync,
        P: Prom<C> + Send + Sync,
    {
        // SAFETY: Lock the root team row (FOR UPDATE) to serialize with
        // concurrent workset creations, preventing resource leaks from
        // worksets (and their subtrees) inserted between the listing and
        // the team delete.
        let team_info = repo
            .advance(context, &TeamStep::get_info_excluded(id))
            .await?;

        let workset_infos = repo
            .advance(
                context,
                &WorksetStep::list_all_infos_by_team_id_excluded(&team_info.id),
            )
            .await?;

        for workset_info in workset_infos {
            WorksetComplex::delete_cascade(
                repo,
                prom,
                context,
                &workset_info.id,
            )
            .await?;
        }

        if let Some(avatar_key) = &team_info.avatar_key
            && team_info.avatar_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();

            let now = OffsetDateTime::now_utc();

            prom.advance(
                context,
                &PromStep::append(
                    &delete_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageTask::Delete {
                        object_key: avatar_key.as_str(),
                    }),
                    &now,
                ),
            )
            .await?;
        }

        repo.advance(context, &TeamStep::delete(&team_info.id))
            .await?;

        Ok(())
    }
}

/// Permission-gate opers for team entities.
pub struct TeamPermComplex;

impl TeamPermComplex {
    /// Verify the user has super-admin privileges required to list all teams.
    /// Returns an `Expected::Perm` error if the user is not a super-admin.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn can_user_reserve_avatar<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn can_user_mark_avatar_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the user has super-admin privileges required to list all teams.
    pub async fn can_user_list_all<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoById<'a>, Error = RegularError>,
    {
        Self::check_user_is_sadmin(proxy, user_id).await
    }

    /// Check whether the user is a super-admin; returns `Perm` error if not.
    async fn check_user_is_sadmin<P>(
        proxy: &mut P,
        user_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoById<'a>, Error = RegularError>,
    {
        let user_info =
            proxy.execute(&UserStep::get_info_by_id(user_id)).await?;

        if !user_info.is_sadmin {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-sadmin-required"),
            });
        }

        Ok(())
    }
}
