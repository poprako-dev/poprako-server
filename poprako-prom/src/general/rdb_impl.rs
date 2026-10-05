//! Typed bindings for a caller-owned local-message table.

// Generated consumer delivery adapters.
mod delivery;
// Generated typed delivery repository operations.
mod repo;
// Generated transactional task writers.
mod writer;

use std::io::Write as _;

use diesel::AsExpression;
use diesel::pg::Pg;
use diesel::serialize::{IsNull, Output, Result as SerializeResult, ToSql};
use diesel::sql_types::Text;

/// Lifecycle status of a local message record in the prom delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = Text)]
pub enum LocalTaskStatus {
    /// Waiting until the task becomes visible and its topic is idle.
    Pending,

    /// Owned by an in-flight attempt with a claim token.
    Processing,

    /// Retained after a fatal failure or an exhausted retry budget.
    Dead,
}

impl LocalTaskStatus {
    /// Returns the stable persisted status name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        //
        match self {
            //
            Self::Pending => "local_message_status:pending",

            Self::Processing => "local_message_status:processing",

            Self::Dead => "local_message_status:dead",
        }
    }
}

impl ToSql<Text, Pg> for LocalTaskStatus {
    // Serializes the persisted lifecycle status.
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> SerializeResult {
        //
        out.write_all(self.as_str().as_bytes())?;

        Ok(IsNull::No)
    }
}

/// Neutral errors converted by the application at its adapter boundary.
#[derive(Debug)]
pub enum PromError {
    /// A task with the same identity already exists.
    AlreadyExists,
    /// A serializable transaction lost a concurrent race.
    ConcurrentConflict,
    /// An operation unexpectedly found no record.
    NotFound,
    /// Serialization, invalid timing, or a permanent database failure.
    Unrecoverable {
        /// Safe diagnostic retained after recording the original source.
        msg: String,
    },
}

impl std::fmt::Display for PromError {
    // Formats the safe diagnostic for error consumers.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //
        match self {
            //
            Self::AlreadyExists => {
                formatter.write_str("prom task already exists")
            }

            Self::ConcurrentConflict => {
                formatter.write_str("prom concurrent conflict")
            }

            Self::NotFound => formatter.write_str("prom task not found"),

            Self::Unrecoverable { msg } => formatter.write_str(msg),
        }
    }
}

impl std::error::Error for PromError {}

/// Records a database failure once and retains its application-neutral class.
#[doc(hidden)]
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub fn diesel_error(source: diesel::result::Error) -> PromError {
    //
    use diesel::result::{DatabaseErrorKind, Error};

    match source {
        //
        Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            information,
        ) => {
            //
            tracing::warn!(
                operation = "persist_prom",
                database_err = "unique violation",
                database_msg = information.message(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "prom database conflict"
            );

            PromError::AlreadyExists
        }

        Error::DatabaseError(
            DatabaseErrorKind::SerializationFailure,
            information,
        ) => {
            //
            tracing::warn!(
                operation = "execute_prom",
                database_err = "serialization failure",
                database_msg = information.message(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "prom database conflict"
            );

            PromError::ConcurrentConflict
        }

        Error::NotFound => {
            //
            tracing::warn!(
                operation = "execute_prom",
                database_err = "not found",
                "unexpected missing prom record"
            );

            PromError::NotFound
        }

        Error::DatabaseError(kind, information) => {
            // Database details can contain the entire private task payload.
            tracing::error!(
                operation = "execute_prom",
                database_err = ?kind,
                database_msg = information.message(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "Diesel database error",
            );

            PromError::Unrecoverable {
                msg: format!("diesel error: {}", information.message()),
            }
        }

        source => {
            //
            tracing::error!(operation = "execute_prom", sdk_err = ?source, "Diesel SDK error");

            PromError::Unrecoverable {
                msg: format!("diesel error: {}", source),
            }
        }
    }
}

/// Declares a transactional writer, delivery repository, and consumer delivery.
///
/// Invoke once per module, importing the generated Diesel table under its own
/// name. The table must have the existing local-message columns and constraints.
/// `nucl` is a concrete coordinator type providing `RdbContext<claim_level>`;
/// its coordinated future must be `Send`. `claim_level` must provide serializable
/// isolation and satisfy `AtLeast<write_level>`. `error` converts `PromError` and
/// the coordinator's transaction errors. Import the type arguments at the
/// invocation site and use their local names.
///
/// Generated `entity` and `repo` modules contain the typed table bindings and
/// delivery operations. The writer uses caller-owned transactions; only the delivery
/// uses the injected coordinator. No task is started by either constructor.
/// Callers need the Diesel, diesel-async, Orchestra, Orchestra-extra, rdb-core,
/// `serde_json`, time, tracing, and uuid dependencies used by the typed expansion.
///
/// ```ignore
/// use crate::part::nucl::{ReptRead, Serial};
/// use crate::part_impl::nucl::rdb_impl::RdbNucl;
/// use crate::result::BaseError;
///
/// rdb_general_prom! {
///     writer: RdbProm,
///     delivery: RdbPromDelivery,
///     repo: RdbPromRepo,
///     table: t_local_message,
///     nucl: RdbNucl<Serial>,
///     write_level: ReptRead,
///     claim_level: Serial,
///     error: BaseError,
/// }
/// ```
#[macro_export]
// Expands application-owned typed task storage and delivery adapters.
macro_rules! rdb_general_prom {
    (
        writer: $writer:ident,
        delivery: $delivery:ident,
        repo: $repo:ident,
        table: $table:ident,
        nucl: $nucl:ty,
        write_level: $write_level:ty,
        claim_level: $claim_level:ty,
        error: $error:ty $(,)?
    ) => {
        type PromWriteLevel = $write_level;
        type PromClaimLevel = $claim_level;
        type PromAppError = $error;
        type PromNucl = $nucl;

        $crate::__general_prom_writer!(
            $writer,
            $table,
            super::PromWriteLevel,
            super::PromAppError
        );
        $crate::__general_prom_repo!(
            $repo,
            $table,
            super::PromWriteLevel,
            super::PromClaimLevel,
            super::PromAppError
        );
        $crate::__general_prom_delivery!(
            $delivery,
            $repo,
            super::PromNucl,
            super::PromClaimLevel,
            super::PromAppError
        );
    };
}
