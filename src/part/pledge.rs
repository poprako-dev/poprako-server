use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::step::Step;

use crate::part::pledge::intention::ImageIntention;
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub mod intention;

#[derive(Serialize, Deserialize)]
pub enum Payload {
    Image(ImageIntention),
}

pub struct Append<'a> {
    pub id: &'a str,

    // TODO: ref.
    pub topic: &'a str,
    pub payload: Payload,

    pub visible_at: &'a OffsetDateTime,
}

impl<'a> Step for Append<'a> {
    type Output = ();
}

pub struct PromStep;

impl PromStep {
    pub fn append<'a>(
        id: &'a str,
        topic: &'a str,
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

pub trait Prom<C>: DeriveTransactional
where
    Self::Transactional: PromTransactional<C>,
{
}

pub trait PromTransactional<C>: for<'a> Advance<Append<'a>, C, Error = RootError> {}
