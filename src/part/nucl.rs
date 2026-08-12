//! Transaction-coordinator boundary error conversion.

use poprako_orchestra::nucl::Error as NuclError;

use crate::result::BaseError;

impl<BE, E> From<NuclError<BE, E>> for BaseError
where
    BE: Into<BaseError>,
    E: Into<BaseError>,
{
    // Converts a Nucl error into an application-level Error, unwrapping the backend or step inner error.
    fn from(value: NuclError<BE, E>) -> Self {
        //
        match value {
            //
            NuclError::Backend(error) => error.into(),

            NuclError::Step(error) => error.into(),
        }
    }
}
