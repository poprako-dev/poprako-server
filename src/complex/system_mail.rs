use uuid::Uuid;

pub struct SystemMailComplex;

impl SystemMailComplex {
    pub fn gen_id() -> String {
        format!("sys_mail-{}", Uuid::now_v7())
    }
}
