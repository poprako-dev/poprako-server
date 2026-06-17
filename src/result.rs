use poprako_transactional::run::result::Error as RunError;

pub enum ExpectedVariant {
    Args,
    Auth,
    Perm,
}

pub enum Error {
    Expected {
        variant: ExpectedVariant,
        message: String,
    },
    Unrecoverable {
        message: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn accept<T>(v: T) -> Result<T> {
    Ok(v)
}

pub type RootError = Error;

pub type RootResult<T> = Result<T>;

impl<E, BE> From<RunError<E, BE>> for Error
where
    E: Into<Error>,
    BE: Into<Error>,
{
    fn from(value: RunError<E, BE>) -> Self {
        match value {
            RunError::Advance(e) => e.into(),
            RunError::Backend(e) => e.into(),
        }
    }
}
