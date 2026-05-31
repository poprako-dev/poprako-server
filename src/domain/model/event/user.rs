#[derive(Clone, Debug)]
pub struct UserSignedUpEvent {
    pub team_id: String,
    pub invitor_id: String,

    pub invitee_qid: String,
}
