use super::{
    RDB_POOL_MAX_SIZE, RDB_POOL_WAIT_TIMEOUT, RdbCore, RdbError, RdbRest,
    pool_get_error,
};
use deadpool::managed::TimeoutType;
use diesel_async::pooled_connection::deadpool::PoolError;

#[test]
fn database_pool_has_low_resource_capacity() -> RdbRest<()> {
    let core = RdbCore::from_database_url(
        "postgres://unused:unused@127.0.0.1/unused",
    )?;

    assert_eq!(core.pool.status().max_size, RDB_POOL_MAX_SIZE);
    assert_eq!(core.pool.timeouts().wait, Some(RDB_POOL_WAIT_TIMEOUT));

    Ok(())
}

#[test]
fn pool_wait_timeout_has_structured_error_variant() {
    assert_eq!(
        RdbError::PoolWaitTimeout.to_string(),
        "timed out waiting for an RDB connection",
    );
}

#[test]
fn only_wait_timeout_uses_wait_timeout_classification() {
    let wait_error = pool_get_error(&PoolError::Timeout(TimeoutType::Wait));

    let create_error = pool_get_error(&PoolError::Timeout(TimeoutType::Create));

    assert!(matches!(wait_error, RdbError::PoolWaitTimeout));
    assert!(matches!(create_error, RdbError::PoolGet { .. }));
}
