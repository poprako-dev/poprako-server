//! Image identity value types shared by page persistence and uploads.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

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
            return Err(D::Error::custom("image hash must be 44 Base64 characters"));
        }

        let decoded = STANDARD
            .decode(&encoded)
            .map_err(|_| D::Error::custom("image hash must use padded RFC 4648 Base64"))?;

        let bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_| D::Error::custom("image hash must decode to 32 bytes"))?;

        let image_hash = Self(bytes);

        if image_hash.to_base64() != encoded {
            return Err(D::Error::custom("image hash must use canonical Base64"));
        }

        Ok(image_hash)
    }
}

/// Supported page image filename extensions and their media types.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImageExtension {
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

impl ImageExtension {
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

impl FromStr for ImageExtension {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_hash_round_trips_canonical_base64() {
        let encoded = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        let image_hash = serde_json::from_str::<ImageHash>(&format!("\"{}\"", encoded)).unwrap();

        assert_eq!(image_hash.to_base64(), encoded);
        assert_eq!(image_hash.bytes(), [0; 32]);
    }

    #[test]
    fn image_hash_rejects_noncanonical_encodings() {
        let invalid_hashes = [
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "__________________________________________8=",
            "00000000000000000000000000000000000000000000",
        ];

        for invalid_hash in invalid_hashes {
            let result = serde_json::from_str::<ImageHash>(&format!("\"{}\"", invalid_hash));

            assert!(result.is_err());
        }
    }

    #[test]
    fn image_extensions_provide_fixed_content_types() {
        assert_eq!(ImageExtension::Png.content_type(), "image/png");
        assert_eq!(ImageExtension::Svg.suffix(), "svg");
        assert!(ImageExtension::parse("exe").is_none());
    }
}
