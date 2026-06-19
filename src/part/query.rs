use async_trait::async_trait;

use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::step::Step;

use crate::result::RootError;

pub mod member;
pub mod member_invitation;
pub mod step;
pub mod team;
pub mod user;
pub mod workset;

#[async_trait]
pub trait Execute<S>
where
    S: Step,
{
    type Error;

    async fn execute(&self, step: S) -> Result<S::Output, Self::Error>;
}

#[async_trait]
pub trait DeriveTransactional {
    type Transactional;

    async fn transactional(&self) -> Self::Transactional;
}

pub fn map_drive_err<E, BE>(err: DriveError<E, BE>) -> RootError
where
    E: Into<RootError>,
    BE: Into<RootError>,
{
    err.into()
}
