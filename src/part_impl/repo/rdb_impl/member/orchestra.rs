use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::member::MemberInfo;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    ListMemberInfosExcluded, UpdateMember,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::member::step_impl::{
    create, delete, find_info_by_user_id_and_team_id, get_info_by_id,
    list_infos, list_infos_by_team_id_excluded, list_infos_by_user_id,
    list_infos_by_user_id_excluded, update_role, update_user_nickname,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<ListMemberInfos<'_>> for RdbRepo {
    // Non-transactional query path for listing member infos.
    //
    // It picks the right query variant based on whether the caller provided
    // filters or a user-only scope.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Route list variants to the corresponding read-only query in the current repo.
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
    // Non-transactional query path for loading one member info by identity.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Resolve one member info by id through a submit-query read path.
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
    // Create a new member row inside the active transaction context.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Invoke `create` step with raw entry payload and return created info.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateMember<'_>,
    ) -> BaseResult<MemberInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateMember<'_>, RdbContext> for RdbRepo {
    // Apply an in-transaction member update request.
    //
    // Supported requests either adjust nickname or change role based on branch.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Branch to nickname/role update path and execute inside the transaction.
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
    // Transactional list path for member info queries.
    //
    // Reuses the same selection modes as non-transactional `run` execution,
    // but executes through an explicit DB connection context.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Execute list query within the transaction and return full member info set.
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
    // Transactional lookup for a member by `(user_id, team_id)`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Find one member-row mapping by both user and team identifiers.
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
    // Transactional lookup for a full member info by id and requested includes.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Fetch a member info with requested include options within the transaction.
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
    // Transactional list for member infos excluding one side of relation.
    //
    // One branch excludes all members for a user, the other excludes a full team.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Execute exclusion-based member list variants inside the same tx context.
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
    // Transactional delete operation for a single member by id.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Delete the member row for the provided identifier from the DB transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteMember<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
