//! Transaction-coordinator boundary error conversion.

use poprako_orchestra::nucl::Error as NuclError;
use poprako_orchestra::{AtLeast, Level};

use crate::result::BaseError;

/// PostgreSQL repeatable-read transaction guarantee.
pub struct ReptRead;

impl Level for ReptRead {}

/// PostgreSQL serializable transaction guarantee.
pub struct Serial;

impl Level for Serial {}

impl AtLeast<ReptRead> for Serial {}

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
