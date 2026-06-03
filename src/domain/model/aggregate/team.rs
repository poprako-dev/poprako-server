use time::OffsetDateTime;

#[cfg_attr(test, derive(Clone))]
pub struct TeamAggr {
    pub id: String,

    pub name: String,
    pub description: String,

    pub avatar_key: String,
    pub avatar_uploaded: bool,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
