pub struct ImageComplex;

impl ImageComplex {
    pub fn generate_delete_id() -> String {
        format!("lm-{}", uuid::Uuid::now_v7())
    }
}
