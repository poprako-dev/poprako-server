use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtRest};

/// Rest returned by `ObjDept` operations.
pub type ObjDeptRest<T> = std::result::Result<T, ObjDeptError>;

// Record safe diagnostics without introducing delivery-layer dependencies.
#[track_caller]
fn record_client_error(variant: &str, message: &str) {
    //
    let origin = std::panic::Location::caller();

    tracing::warn!(
        err_variant = variant,
        err_message = message,
        source_file = origin.file(),
        source_line = origin.line(),
        "object operation rejected",
    );
}

/// Failure returned by an `ObjDept` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjDeptError {
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

    /// A temporarily unavailable dependency can be retried later.
    Unavailable {
        /// Safe diagnostic for the unavailable dependency.
        message: String,
    },

    /// Corrupt state or a permanent dependency failure requires intervention.
    Unrecoverable {
        /// Safe diagnostic for the unrecoverable failure.
        message: String,
    },
}

impl ObjDeptError {
    /// Records and constructs an invalid instruction at its source.
    #[must_use]
    #[track_caller]
    pub fn invalid(message: String) -> Self {
        //
        record_client_error("Invalid", &message);

        Self::Invalid { message }
    }

    /// Records and constructs a conflicting operation at its source.
    #[must_use]
    #[track_caller]
    pub fn conflict(message: String) -> Self {
        //
        record_client_error("Conflict", &message);

        Self::Conflict { message }
    }

    /// Records a retryable failure that has not already been traced by its adapter.
    #[must_use]
    #[track_caller]
    pub fn retryable(message: String) -> Self {
        //
        record_client_error("Retryable", &message);

        Self::Retryable { message }
    }
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
            | Self::Unavailable { message }
            | Self::Unrecoverable { message } => message,
        };

        formatter.write_str(message)
    }
}

impl Error for ObjDeptError {}
