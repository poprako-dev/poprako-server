use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::member::MemberInfo;
use crate::part::repo::oper::member::{CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos, ListMemberInfosExcluded, UpdateMember};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::member::step_impl::{create, delete, find_info_by_user_id_and_team_id, get_info_by_id, list_infos, list_infos_by_team_id_excluded, list_infos_by_user_id, list_infos_by_user_id_excluded, update_role, update_user_nickname};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<ListMemberInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListMemberInfos<'_>,
    ) -> BaseResult<Vec<MemberInfo>> {
        match oper {
            //
            ListMemberInfos::Spec { spec } => {
                submit_query!(self.core, list_infos, spec)
            }

            ListMemberInfos::User { user_id } => {
                submit_query!(self.core, list_infos_by_user_id, user_id)
            }
        }
    }
}

impl Run<GetMemberInfo<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetMemberInfo<'_, '_>,
    ) -> BaseResult<MemberInfo> {
        match oper {
            GetMemberInfo::Id { id, incls } => {
                submit_query!(self.core, get_info_by_id, id, incls)
            }
        }
    }
}

impl Step<CreateMember<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateMember<'_>,
    ) -> BaseResult<MemberInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateMember<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateMember<'_>,
    ) -> BaseResult<()> {
        match oper {
            //
            UpdateMember::UserNickname {
                user_id,
                user_nickname,
            } => {
                update_user_nickname(context.conn(), user_id, user_nickname)
                    .await
            }

            UpdateMember::Role { update } => {
                update_role(context.conn(), update).await
            }
        }
    }
}

impl Step<ListMemberInfos<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListMemberInfos<'_>,
    ) -> BaseResult<Vec<MemberInfo>> {
        match oper {
            //
            ListMemberInfos::Spec { spec } => {
                list_infos(context.conn(), spec).await
            }

            ListMemberInfos::User { user_id } => {
                list_infos_by_user_id(context.conn(), user_id).await
            }
        }
    }
}

impl Step<FindMemberInfo<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindMemberInfo<'_>,
    ) -> BaseResult<Option<MemberInfo>> {
        match oper {
            FindMemberInfo::UserTeam { user_id, team_id } => {
                find_info_by_user_id_and_team_id(
                    context.conn(),
                    user_id,
                    team_id,
                )
                .await
            }
        }
    }
}

impl Step<GetMemberInfo<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetMemberInfo<'_, '_>,
    ) -> BaseResult<MemberInfo> {
        match oper {
            GetMemberInfo::Id { id, incls } => {
                get_info_by_id(context.conn(), id, incls).await
            }
        }
    }
}

impl Step<ListMemberInfosExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListMemberInfosExcluded<'_>,
    ) -> BaseResult<Vec<MemberInfo>> {
        match oper {
            //
            ListMemberInfosExcluded::User { user_id } => {
                list_infos_by_user_id_excluded(context.conn(), user_id).await
            }

            ListMemberInfosExcluded::Team { team_id } => {
                list_infos_by_team_id_excluded(context.conn(), team_id).await
            }
        }
    }
}

impl Step<DeleteMember<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteMember<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
