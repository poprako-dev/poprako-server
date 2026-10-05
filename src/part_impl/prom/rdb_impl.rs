//! Application binding for the reusable general Prom queue.

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod actor_tests;
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod repo_tests;
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod test_shared;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod tests;

use poprako_prom::rdb_general_prom;

use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::BaseError;

rdb_general_prom! {
    writer: RdbProm,
    delivery: RdbPromDelivery,
    repo: RdbPromRepo,
    table: t_local_message,
    nucl: RdbNucl<Serial>,
    write_level: ReptRead,
    claim_level: Serial,
    error: BaseError,
}
