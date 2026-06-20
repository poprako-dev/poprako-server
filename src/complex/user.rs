use uuid::Uuid;

pub struct UserComplex;

impl UserComplex {
    pub fn gen_id() -> String {
        format!("user-{}", Uuid::now_v7())
    }

    // TODO: use.
    pub fn gen_avatar_key(prev_version: Option<&str>) -> String {
        todo!()
    }

    pub fn gen_avatar_delete_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }

    pub fn gen_avatar_check_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }
}
