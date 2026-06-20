pub struct ImageComplex;

// TODO: wtf?
impl ImageComplex {
    pub fn gen_delete_id() -> String {
        format!("lm-{}", uuid::Uuid::now_v7())
    }
}
