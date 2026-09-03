use super::{RDB_POOL_MAX_SIZE, RdbCore, RdbRest};

#[test]
fn database_pool_has_low_resource_capacity() -> RdbRest<()> {
    let core = RdbCore::from_database_url(
        "postgres://unused:unused@127.0.0.1/unused",
    )?;

    assert_eq!(core.pool.status().max_size, RDB_POOL_MAX_SIZE);

    Ok(())
}
