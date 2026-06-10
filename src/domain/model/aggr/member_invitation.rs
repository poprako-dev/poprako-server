use time::OffsetDateTime;

use crate::domain::model::aggr::user::UserAggr;
use crate::domain::model::value::role::RoleMask;

#[cfg_attr(test, derive(Clone))]
pub struct MemberInvitationAggr {
    pub id: String,

    pub invitor_id: String,
    pub invitor: Option<UserAggr>,
    pub team_id: String,

    pub invitee_qid: String,

    pub code: String,
    pub pending: bool,

    pub role_mask: RoleMask,

    pub created_at: OffsetDateTime,
}

impl MemberInvitationAggr {
    pub fn generate_id() -> String {
        format!("member_invitation-{}", uuid::Uuid::now_v7())
    }

    pub fn verify_code(&self, code: &str) -> bool {
        self.code == code
    }
}

#[cfg(test)]
mod tests {
    // verify_code_match(MemberInvitationAggr::verify_code)(positive): verification should pass when the code matches exactly.
    // verify_code_mismatch_empty(MemberInvitationAggr::verify_code)(negative): verification should fail for an empty code.
    // verify_code_mismatch_case(MemberInvitationAggr::verify_code)(negative): verification should fail when letter case differs.
    // verify_code_mismatch_prefix(MemberInvitationAggr::verify_code)(negative): verification should fail when the input has extra characters.

    use super::MemberInvitationAggr;

    use time::OffsetDateTime;

    use crate::domain::model::value::role::RoleFlag;
    use crate::domain::model::value::role::RoleMask;

    fn dummy_aggr(code: &str) -> MemberInvitationAggr {
        let now = OffsetDateTime::now_utc();
        MemberInvitationAggr {
            id: MemberInvitationAggr::generate_id(),
            invitor_id: "invitor-1".into(),
            invitor: None,
            team_id: "team-1".into(),
            invitee_qid: "invitee".into(),
            code: code.into(),
            pending: true,
            role_mask: RoleMask::from(RoleFlag::Admin),
            created_at: now,
        }
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
