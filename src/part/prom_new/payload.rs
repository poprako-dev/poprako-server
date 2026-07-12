use crate::part::prom_new::payload::image::Payload as ImagePayload;

pub mod image;

pub enum Payload {
    Image(ImagePayload),
}
