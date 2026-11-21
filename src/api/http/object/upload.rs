//! HTTP wire objects for presigned upload capabilities.
use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::result::Wire;
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::upload::UploadHeaderView;

/// One header bound into a presigned upload request.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UploadHeaderObj {
    /// Header name.
    pub name: String,

    /// Header value bound into the upload signature.
    pub value: String,
}

impl From<UploadHeaderView> for UploadHeaderObj {
    // Converts one bound header into its wire representation.
    fn from(view: UploadHeaderView) -> Self {
        Self {
            name: view.name,
            value: view.value,
        }
    }
}

/// Presigned capability for one pending image upload.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImageUploadSlotObj {
    /// Presigned PUT URL.
    pub put_url: String,
    /// Monotonic image identity version.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
    /// Headers bound into the PUT signature.
    pub headers: Vec<UploadHeaderObj>,
}

impl From<ImageUploadSlotView> for ImageUploadSlotObj {
    // Converts the presentation view into its wire representation.
    fn from(view: ImageUploadSlotView) -> Self {
        Self {
            put_url: view.put_url,
            image_ver: view.image_ver,
            headers: view.headers.into_iter().map(Into::into).collect(),
        }
    }
}

impl Wire for ImageUploadSlotView {
    // Serializable wire payload produced by this conversion.
    type Target = ImageUploadSlotObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

impl Wire for UploadHeaderView {
    // Serializable wire payload produced by this conversion.
    type Target = UploadHeaderObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
