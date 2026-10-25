//! Chapter artwork object identities and content hashes.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_obj_dept::key::ObjId;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::value::business_id::ChapterId;
use crate::value::image::ImageHash;

/// SHA-256 identity using the same canonical Base64 encoding as image uploads.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(feature = "swagger", schema(value_type = String))]
pub struct ArtworkHash(ImageHash);

impl ArtworkHash {
    /// Borrows the SHA-256 bytes used for object deduplication.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Builds a content identity from stored SHA-256 bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(ImageHash::new(bytes))
    }
}

/// Validated chapter artwork suffix, independent of physical-key layout.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtworkExt(String);

impl ArtworkExt {
    /// Checks the suffix accepted by the artwork upload contract.
    pub fn is_valid(value: &str) -> bool {
        //
        !value.is_empty()
            && value.len() <= 32
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
            })
    }

    /// Validates an artwork suffix before object allocation.
    pub fn new(value: &str) -> ObjDeptRest<Self> {
        <Self as ObjId>::from_owned(value.into())
    }

    /// Borrows the canonical suffix.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ObjId for ArtworkExt {
    // Borrows the validated suffix without allocating.
    fn as_str(&self) -> &str {
        self.as_str()
    }

    // Validates a borrowed suffix at the object boundary.
    fn from_text(value: &str) -> ObjDeptRest<Self> {
        Self::new(value)
    }

    // Retains the owned suffix buffer after validation.
    fn from_owned(value: String) -> ObjDeptRest<Self> {
        //
        if !Self::is_valid(&value) {
            return Err(ObjDeptError::invalid("invalid artwork suffix".into()));
        }

        Ok(Self(value))
    }
}

impl Serialize for ArtworkExt {
    // Serializes the validated suffix as canonical text.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ArtworkExt {
    // Validates the owned suffix during deserialization.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::from_owned(value).map_err(D::Error::custom)
    }
}

/// Business identity of the chapter's single artwork slot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct ChapterArtworkKey {
    /// Stable chapter identifier.
    pub chapter_id: ChapterId,

    /// File suffix, validated before allocation.
    pub ext: ArtworkExt,
}
