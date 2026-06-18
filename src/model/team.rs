use time::OffsetDateTime;

pub struct TeamInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,
    pub workset_next_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct TeamForm {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub struct TeamAvatarReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub avatar_version: i64,
}
