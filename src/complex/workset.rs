use uuid::Uuid;

pub struct WorksetComplex;

impl WorksetComplex {
    pub fn gen_id() -> String {
        format!("workset-{}", Uuid::now_v7())
    }
}
