//! Member invitation use cases.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::member_invitation::{
    MemberInvitationComplex, MemberInvitationPermComplex,
};
use crate::data::member_invitation::{
    CreateMemberInvitationParams, CreateMemberInvitationPayload,
    ListMemberInvitationInfosParams, MemberInvitationInfoVal,
    UpdateMemberInvitationRolesParams,
};
use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationListKind, MemberInvitationListSpec,
    MemberInvitationUpdate,
};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::Payload;
use crate::part::prom::payload::invitation::PurgeExpiredInvitation;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    ListMemberInvitationInfos, UpdateMemberInvitation,
};
use crate::part::repo::oper::user::FindUserInfo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

#[cfg(test)]
mod tests;

const EXPIRY_DELAY: Duration = Duration::from_secs(5 * 24 * 60 * 60);

/// Creates a pending invitation for a team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    params: CreateMemberInvitationParams,
) -> BaseResult<CreateMemberInvitationPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + UserRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    let roles = params.roles;

    MemberInvitationPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let (member_invitation_id, code) = nucl
        .coord(async move |context| {
            //

            let invitee_user_info = repo
                .step(
                    context,
                    &FindUserInfo::Qid {
                        qid: &params.invitee_qid,
                    },
                )
                .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                //

                let invitee_member_info = repo
                    .step(
                        context,
                        &FindMemberInfo::UserTeam {
                            user_id: &invitee_user_info.id,
                            team_id: &params.team_id,
                        },
                    )
                    .await?;

                if invitee_member_info.is_some() {
                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: trl("error-already-team-member"),
                    });
                }
            }

            let member_invitation_id = MemberInvitationComplex::gen_id();

            let code = MemberInvitationComplex::gen_code();

            let member_invitation_entry = MemberInvitationEntry {
                id: member_invitation_id,
                team_id: params.team_id,
                invitor_id: token.user_id,
                invitee_qid: params.invitee_qid,
                code,
                roles,
            };

            let member_invitation_info = repo
                .step(
                    context,
                    &CreateMemberInvitation {
                        entry: &member_invitation_entry,
                    },
                )
                .await?;

            let purge_event = PurgeExpiredInvitation::Member {
                invitation_id: member_invitation_info.id.clone(),
            };

            let purge_payload = Payload::PurgeExpiredInvitation(purge_event);

            let purge_task_id = next_snowflake_id();

            let purge_task = Task {
                id: &purge_task_id,
                payload: &purge_payload,
                delay: Some(EXPIRY_DELAY),
            };

            prom.step(context, &Defer::new(purge_task)).await?;

            accept((member_invitation_info.id, member_invitation_info.code))
        })
        .await?;

    accept(CreateMemberInvitationPayload {
        id: member_invitation_id,
        code,
    })
}

/// Lists invitations for a team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListMemberInvitationInfosParams,
) -> BaseResult<Vec<MemberInvitationInfoVal>>
where
    R: MemberInvitationRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    MemberInvitationPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let kind = match params.pending {
        //
        Some(true) => MemberInvitationListKind::Pending,

        Some(false) => MemberInvitationListKind::Used,

        None => MemberInvitationListKind::All,
    };

    let member_invitation_list_spec = MemberInvitationListSpec {
        team_id: params.team_id,
        kind,
        incl_opt: params.incl_opt,
        offset: params.offset,
        limit: params.limit,
    };

    let member_invitation_infos = repo
        .run(&ListMemberInvitationInfos {
            spec: &member_invitation_list_spec,
        })
        .await?;

    let mut member_invitation_info_vals =
        Vec::with_capacity(member_invitation_infos.len());

    for member_invitation_info in member_invitation_infos {
        member_invitation_info_vals.push(
            MemberInvitationInfoVal::from_model(
                image_pool,
                member_invitation_info,
            )
            .await?,
        );
    }

    accept(member_invitation_info_vals)
}

/// Updates the roles of an invitation.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_roles<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: UpdateMemberInvitationRolesParams,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
{
    MemberInvitationPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetMemberInvitationInfo<'a, 'b>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let member_invitation_update = MemberInvitationUpdate {
            id: params.id,
            roles: params.roles,
        };

        repo.step(
            context,
            &UpdateMemberInvitation::Info {
                update: &member_invitation_update,
            },
        )
        .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

/// Deletes an invitation.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
{
    MemberInvitationPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetMemberInvitationInfo<'a, 'b>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        repo.step(context, &DeleteMemberInvitation { id: &id })
            .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}
