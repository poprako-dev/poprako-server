pub mod user;

use std::sync::LazyLock;

use diesel::r2d2::ConnectionManager;

pub type Conn = diesel::PgConnection;
type Pool = r2d2::Pool<ConnectionManager<Conn>>;

static REPO_POOL: LazyLock<Pool> = LazyLock::new(|| build_pool());

// build_pool creates a connection pool for repositories.
// NOTE: any exception will lead to a panic.
fn build_pool() -> Pool {
    let url = std::env::var("DATABASE_URL").expect("[build_pool] env DATABSEE_URL unset");

    let manager = ConnectionManager::new(url);

    r2d2::Pool::builder()
        .max_size(8)
        .build(manager)
        .expect("[build_pool] failed to build repository pool")
}

// prepare is called at the start of the application
// to make sure the pool is built before any request comes in.
pub fn prepare() {
    // Just to make sure the pool is built before any request comes in.
    LazyLock::force(&REPO_POOL);
}

// `RunError` represents any error that may be encountered
// in a `run_with` function.
#[derive(Debug, thiserror::Error)]
pub enum RunError<E>
where
    E: std::fmt::Display + std::fmt::Debug + Send + 'static,
{
    #[error("pooled connection aquisition error: {0}")]
    Pool(#[from] diesel::r2d2::PoolError),
    #[error("query execution error: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("thread join error: {0}")]
    Thread(#[from] tokio::task::JoinError),

    // No auto wrapping, as it leads to a conflicting impl.
    #[error("business error: {0}")]
    Business(E),
}

// `run_with` simplifies the boilerplate to run database task in
// blocking thread.
async fn run_with<T, F, E>(f: F) -> Result<T, RunError<E>>
where
    T: Send + 'static,
    F: FnOnce(&mut Conn) -> Result<T, RunError<E>> + Send + 'static,
    E: std::fmt::Display + std::fmt::Debug + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let conn = &mut REPO_POOL.get()?;
        f(conn)
    })
    .await?
}
