#[derive(Debug)]
pub struct UserSignedUpEvent {
    pub team_id: String,
    pub invitor_id: String,

    pub invitor_qid: String,
}
