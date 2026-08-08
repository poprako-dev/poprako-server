//! Image identity value types shared by page persistence and uploads.

use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

#[cfg(test)]
mod tests;

/// SHA-256 content hash encoded as canonical padded RFC 4648 Base64.
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
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

    /// Borrows the raw SHA-256 bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns canonical padded RFC 4648 Base64.
    pub fn to_base64(&self) -> String {
        STANDARD.encode(self.0)
    }

    /// Parses a canonical padded RFC 4648 Base64 SHA-256 hash.
    pub fn parse_rfc4648(encoded: &str) -> Option<Self> {
        //
        if encoded.len() != 44 {
            return None;
        }

        let decoded = STANDARD.decode(encoded).ok()?;

        let bytes = TryInto::<[u8; 32]>::try_into(decoded).ok()?;

        let image_hash = Self(bytes);

        match image_hash.to_base64() == encoded {
            //
            true => Some(image_hash),

            false => None,
        }
    }
}

impl Serialize for ImageHash {
    // Serialize SHA-256 hash as padded RFC 4648 base64 string.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

impl<'de> Deserialize<'de> for ImageHash {
    // Deserialize image hash from canonical padded RFC 4648 base64.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;

        Self::parse_rfc4648(&encoded).ok_or_else(|| {
            //
            D::Error::custom(
                "image hash must be canonical padded RFC 4648 Base64 for 32 bytes",
            )
        })
    }
}

/// Supported page image filename extensions and their media types.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImageExt {
    //
    /// The JPEG image format.
    Jpg,

    /// The JPEG image format (explicit `image/jpeg` content type).
    Jpeg,

    /// The PNG image format.
    Png,

    /// The GIF image format.
    Gif,

    /// The WebP image format.
    Webp,

    /// The SVG image format.
    Svg,

    /// The AVIF image format.
    Avif,

    /// The BMP image format.
    Bmp,

    /// The TIFF image format.
    Tif,

    /// The TIFF image format (explicit `image/tiff` content type).
    Tiff,
}

impl ImageExt {
    /// Parses a supported lowercase object-key suffix.
    pub fn parse(value: &str) -> Option<Self> {
        //
        match value {
            //
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
        //
        match self {
            //
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
        //
        match self {
            //
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
    // Error returned when an image extension string cannot be parsed.
    type Err = ();

    // Parse image extension from a string, returning error on failure.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or(())
    }
}
