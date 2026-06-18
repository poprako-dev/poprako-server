pub struct UserActivePayload {
    pub user_id: String,
}

pub struct UserSignedUpPayload {
    pub team_id: String,
    pub invitor_id: String,
    pub invitee_qid: String,
}
