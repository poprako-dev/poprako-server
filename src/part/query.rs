use async_trait::async_trait;
use poprako_transactional::run::result::Error as RunError;
use poprako_transactional::step::Step;

use crate::result::RootError;

pub mod action;

pub mod member;
pub mod user;

#[async_trait]
pub trait Execute<S>
where
    S: Step,
{
    type Error;

    async fn execute(&self, step: S) -> Result<S::Output, Self::Error>;
}

pub trait DeriveTransactional {
    type Transactional;

    async fn transactional(&self) -> Self::Transactional;
}

pub fn map_run_err<E, BE>(err: RunError<E, BE>) -> RootError
where
    E: Into<RootError>,
    BE: Into<RootError>,
{
    err.into()
}
