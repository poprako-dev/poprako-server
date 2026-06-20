use uuid::Uuid;

pub struct TeamComplex;

impl TeamComplex {
    pub fn generate_id() -> String {
        format!("team-{}", Uuid::now_v7())
    }

    pub fn generate_avatar_delete_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }

    pub fn generate_avatar_check_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }
}
