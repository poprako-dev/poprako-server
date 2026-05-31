use time::OffsetDateTime;

use crate::domain::model::aggregate::PrivateMarker;
use crate::domain::model::aggregate::user::UserAggr;
use crate::domain::model::value::role::RoleMask;

pub struct MemberInvitationAggr {
    pub id: String,

    pub invitor_id: String,
    pub invitor: Option<UserAggr>,
    pub team_id: String,

    pub invitee_qid: String,

    pub code: String,
    pub pending: bool,

    pub roles: RoleMask,

    pub created_at: OffsetDateTime,

    /// Private marker to forbid struct literal construction outside this module.
    _p: PrivateMarker,
}

impl MemberInvitationAggr {
    pub fn generate_id() -> String {
        format!("member_invitation-{}", uuid::Uuid::now_v7())
    }

    pub fn verify_code(&self, code: &str) -> bool {
        self.code == code
    }

    pub fn new(
        id: String,
        invitor_id: String,
        invitor: Option<UserAggr>,
        team_id: String,
        invitee_qid: String,
        code: String,
        pending: bool,
        roles: RoleMask,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            invitor_id,
            invitor,
            team_id,
            invitee_qid,
            code,
            pending,
            roles,
            created_at,
            _p: PrivateMarker,
        }
    }
}
