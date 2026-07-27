use crate::model::member::MemberInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    BatchByIds, Incl, TeamByIds, UserByIds, populate,
};
use crate::part_impl::shared::RdbConn;
use crate::result::BaseRest;
use crate::value::incl::expand_incl_opts;
use crate::value::member::MemberInclOpt;

preloadable! {
    owner: MemberInfo,
    opt: MemberInclOpt,
    populate: populate_member_incls,
    variants: {
        User => UserByIds {
            resolve: path [] => user_id,
            inject: path [] => user,
        },
        Team => TeamByIds {
            resolve: path [] => team_id,
            inject: path [] => team,
        },
    },
}
