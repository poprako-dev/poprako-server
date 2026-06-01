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
    _m: PrivateMarker,
}

impl MemberInvitationAggr {
    pub fn generate_id() -> String {
        format!("member_invitation-{}", uuid::Uuid::now_v7())
    }

    pub fn verify_code(&self, code: &str) -> bool {
        self.code == code
    }

    #[allow(clippy::too_many_arguments)]
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
            _m: PrivateMarker,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dummy_aggr(code: &str) -> MemberInvitationAggr {
        let now = OffsetDateTime::now_utc();
        MemberInvitationAggr::new(
            MemberInvitationAggr::generate_id(),
            "invitor-1".into(),
            None,
            "team-1".into(),
            "invitee".into(),
            code.into(),
            true,
            RoleMask::from(crate::domain::model::value::role::RoleFlag::Admin),
            now,
        )
    }

    #[test]
    fn verify_code_match() {
        let aggr = dummy_aggr("ABC123");
        assert!(aggr.verify_code("ABC123"));
    }

    #[test]
    fn verify_code_mismatch_empty() {
        let aggr = dummy_aggr("ABC123");
        assert!(!aggr.verify_code(""));
    }

    #[test]
    fn verify_code_mismatch_case() {
        let aggr = dummy_aggr("ABC123");
        assert!(!aggr.verify_code("abc123"));
    }

    #[test]
    fn verify_code_mismatch_prefix() {
        let aggr = dummy_aggr("ABC123");
        assert!(!aggr.verify_code("ABC1234"));
    }
}
