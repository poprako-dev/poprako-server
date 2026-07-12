pub enum ResourceKind {
    UserAvatar,
    TeamAvatar,
    ComicCover,
    PageImage,
}

pub enum Payload {
    CheckUpload {
        resource_kind: ResourceKind,
        resource_id: String,
        object_key: String,
        version: u32,
    },
    Delete {
        object_key: String,
    },
}
