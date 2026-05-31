use time::OffsetDateTime;

use crate::domain::model::aggregate::PrivateMarker;

pub struct TeamAggr {
    pub id: String,

    pub name: String,
    pub description: String,

    pub avatar_key: String,
    pub avatar_uploaded: bool,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl TeamAggr {
    pub fn new(
        id: String,
        name: String,
        description: String,
        avatar_key: String,
        avatar_uploaded: bool,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            name,
            description,
            avatar_key,
            avatar_uploaded,
            created_at,
            updated_at,
            _m: PrivateMarker,
        }
    }
}
