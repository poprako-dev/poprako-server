use url::Url;

/// Read URLs generated for one physical object generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjUrls {
    //
    /// URL for the original object.
    pub origin_url: Url,
    /// Optional URL for a storage-provided thumbnail rendition.
    pub thumbnail_url: Option<Url>,
}
