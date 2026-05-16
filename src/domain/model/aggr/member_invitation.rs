use time::OffsetDateTime;

use crate::domain::model::aggr::user::User;
use crate::domain::model::value::role::RoleMask;

pub struct MemberInvitation {
    pub id: String,

    pub invitor_id: String,
    pub invitor: Option<User>,
    pub team_id: String,

    pub invitee_qid: String,

    pub code: String,
    pub pending: bool,

    pub role_mask: RoleMask,

    pub created_at: OffsetDateTime,
}
