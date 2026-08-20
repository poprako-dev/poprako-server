//! Announcement use cases — list and create team announcements.

#[cfg(test)]
// Unit tests for announcement usecase behavior.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::announcement::{
    AnnouncementComplex, AnnouncementPermComplex,
};
use crate::data::instr::announcement::{
    CreateAnnouncementInstr, ListAnnouncementInfosInstr,
};
use crate::data::val::announcement::CreateAnnouncementVal;
use crate::data::view::announcement::AnnouncementInfoView;
use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::announcement::AnnouncementEntry;
use crate::part::image::ImagePool;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::announcement::AnnouncementRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Lists announcements under a team.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListAnnouncementInfosInstr,
) -> BaseRest<Vec<AnnouncementInfoView>>
where
    C: Context,
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    let announcement_list_spec = Into::<AnnouncementListSpec>::into(instr);

    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &announcement_list_spec.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    AnnouncementPermComplex::ensure_user_can_list_infos(&member_info)?;

    let announcement_infos = ListAnnouncementInfos {
        spec: &announcement_list_spec,
    }
    .run_on(repo)
    .await?;

    let mut announcement_info_vals =
        Vec::with_capacity(announcement_infos.len());

    for announcement_info in announcement_infos {
        //
        announcement_info_vals.push(
            AnnouncementInfoView::from_model(image_pool, announcement_info)
                .await?,
        );
    }

    accept(announcement_info_vals)
}

/// Creates an announcement under a team.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateAnnouncementInstr,
) -> BaseRest<CreateAnnouncementVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: AnnouncementRepo<C> + MemberRepo<C> + Send + Sync,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    AnnouncementPermComplex::ensure_user_can_create(&member_info)?;

    let announcement_info = nucl
        .coord(async move |context| {
            //
            let announcement_entry = AnnouncementEntry {
                id: AnnouncementComplex::gen_id(),
                team_id: instr.team_id,
                user_id: token.user_id,
                title: instr.title,
                content: instr.content,
            };

            CreateAnnouncement {
                entry: &announcement_entry,
            }
            .step_on(repo, context)
            .await
        })
        .await?;

    accept(CreateAnnouncementVal {
        id: announcement_info.id,
    })
}
