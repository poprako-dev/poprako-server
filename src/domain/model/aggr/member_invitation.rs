use time::OffsetDateTime;

use crate::domain::model::aggr::user::User;
use crate::domain::model::val::role::RoleMask;

pub struct MemberInvitation {
    pub id: String,

    pub invitor_id: String,
    pub invitor: Option<User>,
    pub team_id: String,

    pub invitee_qid: String,

    pub code: String,
    pub pending: bool,

    pub roles: RoleMask,

    pub created_at: OffsetDateTime,
}

impl MemberInvitation {
    pub fn verify_code(&self, code: &str) -> bool {
        self.code == code
    }
}
