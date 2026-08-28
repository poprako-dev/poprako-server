//! Page-manifest reservation response data.

use crate::value::image::{ImageExt, ImageHash};

/// Object-storage reservation for one page upload.
pub struct PageUploadReservation {
    //
    /// Object-storage key reserved for the page image.
    object_key: String,
    /// Size of the image upload in bytes.
    new_byte_len: u64,
}

impl PageUploadReservation {
    /// Creates an object-storage reservation for one page upload.
    pub const fn new(object_key: String, new_byte_len: u64) -> Self {
        //
        Self {
            object_key,
            new_byte_len,
        }
    }

    /// Returns the object-storage key without consuming the reservation.
    pub fn object_key(&self) -> &str {
        &self.object_key
    }

    /// Returns the storage key and upload size.
    pub fn into_parts(self) -> (String, u64) {
        (self.object_key, self.new_byte_len)
    }
}

/// Reservation data returned to the parent page-reservation workflow.
pub struct PageReservation {
    //
    /// Identifier of the reserved page.
    page_id: String,
    /// Zero-based page position returned to the caller.
    index: u32,
    /// Optional upload slot data for a new image.
    upload: Option<PageUploadReservation>,
    /// Image version used by the upload confirmation task.
    image_version: u32,
    /// Content hash of the reserved image.
    image_hash: ImageHash,
    /// File extension of the reserved image.
    ext: ImageExt,
}

impl PageReservation {
    /// Creates the complete reservation for one manifest page.
    pub const fn new(
        page_id: String,
        index: u32,
        upload: Option<PageUploadReservation>,
        image_version: u32,
        image_hash: ImageHash,
        ext: ImageExt,
    ) -> Self {
        //
        Self {
            page_id,
            index,
            upload,
            image_version,
            image_hash,
            ext,
        }
    }

    /// Returns data needed to schedule upload verification when present.
    pub fn upload_check(&self) -> Option<(&str, &str, u32)> {
        //
        self.upload.as_ref().map(|upload| {
            //
            (
                self.page_id.as_str(),
                upload.object_key(),
                self.image_version,
            )
        })
    }

    /// Returns the reservation data needed to build the response.
    pub fn into_parts(
        self,
    ) -> (
        String,
        u32,
        Option<PageUploadReservation>,
        u32,
        ImageHash,
        ImageExt,
    ) {
        //
        (
            self.page_id,
            self.index,
            self.upload,
            self.image_version,
            self.image_hash,
            self.ext,
        )
    }
}
