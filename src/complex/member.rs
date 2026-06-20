use uuid::Uuid;

pub struct MemberComplex;

impl MemberComplex {
    pub fn generate_id() -> String {
        format!("member-{}", Uuid::now_v7())
    }
}
