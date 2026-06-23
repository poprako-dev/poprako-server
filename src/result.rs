use poprako_transactional::drive::result::Error as DriveError;

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

impl<E, BE> From<DriveError<E, BE>> for Error
where
    E: Into<Error>,
    BE: Into<Error>,
{
    fn from(value: DriveError<E, BE>) -> Self {
        match value {
            DriveError::Advance(e) => e.into(),
            DriveError::Backend(e) => e.into(),
        }
    }
}
