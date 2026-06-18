use poprako_transactional::{advance::Advance, step::Step};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::model::local_message::ImageLocalMessage;
use crate::result::RootError;

#[derive(Serialize, Deserialize)]
pub enum Payload {
    Image(ImageLocalMessage),
}

pub struct Append<'a> {
    pub id: &'a str,

    pub topic: String,
    pub payload: Payload,

    pub visible_at: &'a OffsetDateTime,
}

impl<'a> Step for Append<'a> {
    type Output = ();
}

pub trait Pledge<H>: for<'a> Advance<Append<'a>, H, Error = RootError> {}
