use crate::value::image::ImageKind;

/// Identity carried by an image-verification task.
#[derive(Clone, Copy)]
pub struct ImageIdentity<'a> {
    //
    /// Kind of resource represented by the image.
    kind: ImageKind,

    /// Identifier of the represented resource.
    resource_id: &'a str,

    /// Object-storage key for the image.
    object_key: &'a str,

    /// Current image version for the resource.
    version: u32,
}

impl<'a> ImageIdentity<'a> {
    /// Builds an image identity from its queued payload fields.
    pub const fn new(
        kind: ImageKind,
        resource_id: &'a str,
        object_key: &'a str,
        version: u32,
    ) -> Self {
        //
        Self {
            kind,
            resource_id,
            object_key,
            version,
        }
    }

    /// Returns the kind of resource represented by the image.
    pub const fn kind(&self) -> ImageKind {
        self.kind
    }

    /// Returns the resource identifier.
    pub const fn resource_id(&self) -> &'a str {
        self.resource_id
    }

    /// Returns the object-storage key.
    pub const fn object_key(&self) -> &'a str {
        self.object_key
    }

    /// Returns the resource image version.
    pub const fn version(&self) -> u32 {
        self.version
    }
}
