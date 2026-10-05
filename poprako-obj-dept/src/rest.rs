use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtRest};

/// Rest returned by `ObjDept` operations.
pub type ObjDeptRest<T> = std::result::Result<T, ObjDeptError>;

// Record safe diagnostics without introducing delivery-layer dependencies.
#[track_caller]
fn record_client_error(variant: &str, msg: &str) {
    //
    let origin = std::panic::Location::caller();

    tracing::warn!(
        err_variant = variant,
        err_msg = msg,
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
        msg: String,
    },

    /// The current object changed while applying the requested operation.
    Conflict {
        /// Safe diagnostic for the conflicting state.
        msg: String,
    },

    /// A transient dependency failure can be retried without operator repair.
    Retryable {
        /// Safe diagnostic for the retryable failure.
        msg: String,
    },

    /// A temporarily unavailable dependency can be retried later.
    Unavailable {
        /// Safe diagnostic for the unavailable dependency.
        msg: String,
    },

    /// Corrupt state or a permanent dependency failure requires intervention.
    Unrecoverable {
        /// Safe diagnostic for the unrecoverable failure.
        msg: String,
    },
}

impl ObjDeptError {
    /// Records and constructs an invalid instruction at its source.
    #[must_use]
    #[track_caller]
    pub fn invalid(msg: String) -> Self {
        //
        record_client_error("Invalid", &msg);

        Self::Invalid { msg }
    }

    /// Records and constructs a conflicting operation at its source.
    #[must_use]
    #[track_caller]
    pub fn conflict(msg: String) -> Self {
        //
        record_client_error("Conflict", &msg);

        Self::Conflict { msg }
    }

    /// Records a retryable failure that has not already been traced by its adapter.
    #[must_use]
    #[track_caller]
    pub fn retryable(msg: String) -> Self {
        //
        record_client_error("Retryable", &msg);

        Self::Retryable { msg }
    }
}

impl Display for ObjDeptError {
    // Formats the safe diagnostic for standard error consumers.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtRest {
        //
        let msg = match self {
            //
            Self::Invalid { msg }
            | Self::Conflict { msg }
            | Self::Retryable { msg }
            | Self::Unavailable { msg }
            | Self::Unrecoverable { msg } => msg,
        };

        formatter.write_str(msg)
    }
}

impl Error for ObjDeptError {}
