use serde::Deserialize;

#[derive(Deserialize)]
pub enum MemberInclOpt {
    User,
    Team,
}
