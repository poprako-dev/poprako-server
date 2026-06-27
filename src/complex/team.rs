//! Complex-domain operations for team entities: identity and avatar-storage key
//! generation, cascading deletion, and permission checks.

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::util::check_user_is_team_admin;
use crate::complex::workset::WorksetComplex;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::member::FindByUserTeamId;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::{GetInfoById, UserStep};
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::team::TeamRepoTransactional;
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;

/// Domain operations for team entities.
pub struct TeamComplex;

impl TeamComplex {
    /// Generate a unique, time-ordered team identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Generate the object-storage key for a team avatar image.
    pub fn gen_avatar_key(id: &str, avatar_version: i64, file_ext: &str) -> String {
        format!("team_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }

    /// Recursively delete a team and all owned resources: enqueues avatar-image
    /// deletion, cascades into workset deletion, then deletes the team record.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: TeamRepoTransactional<C>
            + WorksetRepoTransactional<C>
            + ComicRepoTransactional<C>
            + Send
            + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let team_info = repo
            .advance(context, &TeamStep::get_info_excluded(id))
            .await?;

        let workset_infos = repo
            .advance(context, &WorksetStep::list_infos_by_team_id_excluded(id))
            .await?;

        for workset_info in workset_infos {
            WorksetComplex::delete_cascade(repo, prom, context, &workset_info.id).await?;
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
                    Payload::Image(ImageIntention::Delete {
                        object_key: avatar_key.clone(),
                    }),
                    &now,
                ),
            )
            .await?;
        }

        repo.advance(context, &TeamStep::delete(id)).await?;

        Ok(())
    }
}

/// Permission-gate operations for team entities.
pub struct TeamPermComplex;

impl TeamPermComplex {
    /// Verify the user has super-admin privileges required to list all teams.
    /// Returns an `Expected::Perm` error if the user is not a super-admin.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_reserve_avatar<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_mark_avatar_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_delete<P>(proxy: &mut P, user_id: &str, team_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_list_all<P>(proxy: &mut P, user_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoById<'a>, Error = RootError>,
    {
        Self::check_user_is_sadmin(proxy, user_id).await
    }

    async fn check_user_is_sadmin<P>(proxy: &mut P, user_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoById<'a>, Error = RootError>,
    {
        let user_info = proxy.execute(&UserStep::get_info_by_id(user_id)).await?;

        if !user_info.is_sadmin {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-sadmin-required"),
            });
        }

        accept(())
    }
}
