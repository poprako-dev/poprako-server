pub struct ImageComplex;

impl ImageComplex {
    pub fn gen_delete_id() -> String {
        format!("lm-{}", uuid::Uuid::now_v7())
    }

    pub fn gen_check_id() -> String {
        format!("lm-{}", uuid::Uuid::now_v7())
    }
}
