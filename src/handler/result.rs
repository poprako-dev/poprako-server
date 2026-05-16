use crate::util::rename::StdResult;

#[derive(Debug)]
pub enum Error {}

pub type Result<T> = StdResult<T, Error>;
