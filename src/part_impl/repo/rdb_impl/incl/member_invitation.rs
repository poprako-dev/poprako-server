use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::UserByIds;
use crate::value::member_invitation::MemberInvitationInclOpt;

preloadable! {
    owner: MemberInvitationInfo,
    opt: MemberInvitationInclOpt,
    populate: populate_member_invitation_incls,
    variants: {
        Invitor => UserByIds {
            resolve: path [] => invitor_id,
            inject: path [] => invitor,
        },
    },
}
