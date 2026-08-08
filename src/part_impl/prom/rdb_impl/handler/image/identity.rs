use crate::part::prom::payload::image::ResourceKind;

/// Identity carried by an image-verification task.
#[derive(Clone, Copy)]
pub struct ImageIdentity<'a> {
    /// The kind of resource represented by the image.
    pub kind: ResourceKind,

    /// The resource identifier.
    pub resource_id: &'a str,

    /// The object-storage key.
    pub object_key: &'a str,

    /// The resource image version.
    pub version: u32,
}
