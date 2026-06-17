pub enum ErrorVariant {}

pub enum Error {
    Expected {
        variant: ErrorVariant,
        message: String,
    },
    Unrecoverable {
        message: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub type ScopeError = Error;

pub type ScopeResult<T> = Result<T>;
