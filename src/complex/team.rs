use uuid::Uuid;

pub struct TeamComplex;

impl TeamComplex {
    pub fn gen_id() -> String {
        format!("team-{}", Uuid::now_v7())
    }

    pub fn gen_avatar_delete_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }

    pub fn gen_avatar_check_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }

    pub fn gen_avatar_key(id: &str, avatar_version: i64, file_ext: &str) -> String {
        format!("team_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }
}
