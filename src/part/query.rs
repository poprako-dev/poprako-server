use poprako_transactional::run::result::Error as RunError;

use crate::result::RootError;

pub mod action;

pub mod member;
pub mod user;

pub fn map_run_err<E, BE>(err: RunError<E, BE>) -> RootError
where
    E: Into<RootError>,
    BE: Into<RootError>,
{
    err.into()
}
