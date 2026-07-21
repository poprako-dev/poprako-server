//! Image identity value types shared by page persistence and uploads.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[cfg(test)]
mod tests;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// SHA-256 content hash encoded as canonical padded RFC 4648 Base64.
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ImageHash([u8; 32]);

impl ImageHash {
    /// Builds a hash from its raw SHA-256 bytes.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw SHA-256 bytes.
    pub fn bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Returns canonical padded RFC 4648 Base64.
    pub fn to_base64(&self) -> String {
        STANDARD.encode(self.0)
    }
}

impl Serialize for ImageHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

impl<'de> Deserialize<'de> for ImageHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;

        if encoded.len() != 44 {
            return Err(D::Error::custom(
                "image hash must be 44 Base64 characters",
            ));
        }

        let decoded = STANDARD.decode(&encoded).map_err(|_| {
            D::Error::custom("image hash must use padded RFC 4648 Base64")
        })?;

        let bytes: [u8; 32] = decoded.try_into().map_err(|_| {
            D::Error::custom("image hash must decode to 32 bytes")
        })?;

        let image_hash = Self(bytes);

        if image_hash.to_base64() != encoded {
            return Err(D::Error::custom(
                "image hash must use canonical Base64",
            ));
        }

        Ok(image_hash)
    }
}

/// Supported page image filename extensions and their media types.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImageExt {
    Jpg,
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
    Avif,
    Bmp,
    Tif,
    Tiff,
}

impl ImageExt {
    /// Parses a supported lowercase object-key suffix.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "jpg" => Some(Self::Jpg),
            "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::Webp),
            "svg" => Some(Self::Svg),
            "avif" => Some(Self::Avif),
            "bmp" => Some(Self::Bmp),
            "tif" => Some(Self::Tif),
            "tiff" => Some(Self::Tiff),
            _ => None,
        }
    }

    /// Returns the lowercase object-key suffix.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Svg => "svg",
            Self::Avif => "avif",
            Self::Bmp => "bmp",
            Self::Tif => "tif",
            Self::Tiff => "tiff",
        }
    }

    /// Returns the media type bound into an upload signature.
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Jpg | Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Svg => "image/svg+xml",
            Self::Avif => "image/avif",
            Self::Bmp => "image/bmp",
            Self::Tif | Self::Tiff => "image/tiff",
        }
    }
}

impl FromStr for ImageExt {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or(())
    }
}

