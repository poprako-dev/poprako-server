use time::OffsetDateTime;
use uuid::Uuid;

pub struct SysMail {
    pub id: String,

    pub receiver_id: String,
    pub read: bool,

    pub title: String,
    pub content: String,

    pub created_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct SysMailCre {
    pub id: String,

    pub receiver_id: String,

    pub title: String,
    pub content: String,
}

impl SysMailCre {
    /// Returns a new [`SysMailCre`] with a generated ID.
    pub fn new(receiver_id: String, title: String, content: String) -> Self {
        Self {
            id: format!("sys_mail-{}", Uuid::now_v7()),
            receiver_id,
            title,
            content,
        }
    }
}
