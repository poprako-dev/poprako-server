use url::Url;

/// Selects the read URLs generated for one object operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjUrlSpec {
    //
    /// Whether to generate the original object URL.
    origin: bool,
    /// Whether to generate the optimized image URL.
    optimized: bool,
    /// Whether to generate the thumbnail image URL.
    thumbnail: bool,
}

impl ObjUrlSpec {
    /// Includes the original object URL.
    #[must_use]
    pub const fn with_origin(mut self) -> Self {
        //
        self.origin = true;

        self
    }

    /// Includes the optimized image URL.
    #[must_use]
    pub const fn with_optimized(mut self) -> Self {
        //
        self.optimized = true;

        self
    }

    /// Includes the thumbnail image URL.
    #[must_use]
    pub const fn with_thumbnail(mut self) -> Self {
        //
        self.thumbnail = true;

        self
    }

    /// Reports whether the original object URL is selected.
    #[must_use]
    pub const fn includes_origin(self) -> bool {
        self.origin
    }

    /// Reports whether the optimized image URL is selected.
    #[must_use]
    pub const fn includes_optimized(self) -> bool {
        self.optimized
    }

    /// Reports whether the thumbnail image URL is selected.
    #[must_use]
    pub const fn includes_thumbnail(self) -> bool {
        self.thumbnail
    }

    /// Reports whether no object URL is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.origin && !self.optimized && !self.thumbnail
    }
}

/// Read URLs generated for one physical object generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjUrls {
    //
    /// Optional URL for the original object.
    pub origin_url: Option<Url>,
    /// Optional URL for a storage-provided optimized rendition.
    pub optimized_url: Option<Url>,
    /// Optional URL for a storage-provided thumbnail rendition.
    pub thumbnail_url: Option<Url>,
}
