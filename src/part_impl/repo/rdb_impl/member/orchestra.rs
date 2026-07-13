use poprako_orchestra::{Run, Step};

use crate::model::member::MemberInfo;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    ListMemberInfosExcluded, UpdateMember,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::member::{
    create, delete, find_info_by_user_id_and_team_id, get_info_by_id,
    list_infos, list_infos_by_user_id, list_infos_by_user_id_excluded,
    update_role, update_user_nickname,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{RegularError, RegularResult};

impl<'a> Run<ListMemberInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &ListMemberInfos<'a>,
    ) -> RegularResult<Vec<MemberInfo>> {
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

impl<'a, 'b> Run<GetMemberInfo<'a, 'b>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetMemberInfo<'a, 'b>,
    ) -> RegularResult<MemberInfo> {
        match oper {
            GetMemberInfo::Id { id, incls } => {
                submit_query!(self.core, get_info_by_id, id, incls)
            }
        }
    }
}

impl<'a> Step<CreateMember<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateMember<'a>,
    ) -> RegularResult<MemberInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<UpdateMember<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateMember<'a>,
    ) -> RegularResult<()> {
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

impl<'a> Step<ListMemberInfos<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListMemberInfos<'a>,
    ) -> RegularResult<Vec<MemberInfo>> {
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

impl<'a> Step<FindMemberInfo<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindMemberInfo<'a>,
    ) -> RegularResult<Option<MemberInfo>> {
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

impl<'a, 'b> Step<GetMemberInfo<'a, 'b>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetMemberInfo<'a, 'b>,
    ) -> RegularResult<MemberInfo> {
        match oper {
            GetMemberInfo::Id { id, incls } => {
                get_info_by_id(context.conn(), id, incls).await
            }
        }
    }
}

impl<'a> Step<ListMemberInfosExcluded<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListMemberInfosExcluded<'a>,
    ) -> RegularResult<Vec<MemberInfo>> {
        match oper {
            ListMemberInfosExcluded::User { user_id } => {
                list_infos_by_user_id_excluded(context.conn(), user_id).await
            }
        }
    }
}

impl<'a> Step<DeleteMember<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteMember<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), oper.id).await
    }
}
