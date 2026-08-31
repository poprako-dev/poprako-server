use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtRest};

/// Rest returned by `ObjDept` operations.
pub type ObjDeptRest<T> = std::result::Result<T, ObjDeptError>;

/// Failure returned by an `ObjDept` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjDeptError {
    //
    /// The supplied object instruction is invalid.
    Invalid {
        /// Safe diagnostic for the invalid instruction.
        message: String,
    },

    /// The current object changed while applying the requested operation.
    Conflict {
        /// Safe diagnostic for the conflicting state.
        message: String,
    },

    /// A transient dependency failure can be retried without operator repair.
    Retryable {
        /// Safe diagnostic for the retryable failure.
        message: String,
    },

    /// Corrupt state or a permanent dependency failure requires intervention.
    Unrecoverable {
        /// Safe diagnostic for the unrecoverable failure.
        message: String,
    },
}

impl Display for ObjDeptError {
    // Formats the safe diagnostic for standard error consumers.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtRest {
        //
        let message = match self {
            //
            Self::Invalid { message }
            | Self::Conflict { message }
            | Self::Retryable { message }
            | Self::Unrecoverable { message } => message,
        };

        formatter.write_str(message)
    }
}

impl Error for ObjDeptError {}
