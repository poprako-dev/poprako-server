use crate::util::rename::StdRetVal;

#[derive(Debug)]
pub enum Error {}

pub type Result<T> = StdRetVal<T, Error>;
