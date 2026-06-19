use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::step::Step;

use crate::part::intention::ImageLocalMessage;
use crate::result::RootError;

#[derive(Serialize, Deserialize)]
pub enum Payload {
    Image(ImageLocalMessage),
}

pub struct Append<'a> {
    pub id: &'a str,

    // TODO: ref.
    pub topic: String,
    pub payload: Payload,

    pub visible_at: &'a OffsetDateTime,
}

impl<'a> Step for Append<'a> {
    type Output = ();
}

pub struct PledgeStep;

impl PledgeStep {
    pub fn append<'a>(
        id: &'a str,
        topic: String,
        payload: Payload,
        visible_at: &'a OffsetDateTime,
    ) -> Append<'a> {
        Append {
            id,
            topic,
            payload,
            visible_at,
        }
    }
}

pub trait Pledge<H>: for<'a> Advance<Append<'a>, H, Error = RootError> {}
