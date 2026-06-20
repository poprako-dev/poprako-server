use async_trait::async_trait;
use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as ScopedError;
use poprako_transactional::step::Step;
use poprako_transactional::util::AsyncFnMark;

use diesel_async::AnsiTransactionManager;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::TransactionManager;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{Object, Pool};

// ---- Domain: Step definitions ----

pub struct DecreaseProduct {
    pub product_id: i32,
    pub quantity: i32,
}

impl Step for DecreaseProduct {
    type Output = ();
}

pub struct CreateOrder {
    pub user_id: i32,
    pub product_id: i32,
    pub quantity: i32,
}

impl Step for CreateOrder {
    type Output = ();
}

// ---- Usecase (fully generic — zero infra knowledge) ----

async fn run_order_usecase<M, C, E, D, A>(
    backend: &M,
    decrease_adv: D,
    create_adv: A,
    product_id: i32,
    user_id: i32,
    quantity: i32,
) -> Result<(), ScopedError<E, M::Error>>
where
    M: Drive<C>,
    E: Send,
    D: Advance<DecreaseProduct, C> + Send,
    A: Advance<CreateOrder, C> + Send,
    E: From<D::Error> + From<A::Error>,
    C: Send,
{
    backend
        .with_context::<(), E, _>(async move |context| {
            decrease_adv
                .advance(
                    context,
                    &DecreaseProduct {
                        product_id,
                        quantity,
                    },
                )
                .await?;

            create_adv
                .advance(
                    context,
                    &CreateOrder {
                        user_id,
                        product_id,
                        quantity,
                    },
                )
                .await?;

            Ok(())
        })
        .await
}

// ---- Infra: diesel_async ----

type Conn = Object<AsyncPgConnection>;

pub struct PgContext(Conn);

impl PgContext {
    async fn commit(mut self) -> Result<(), diesel::result::Error> {
        AnsiTransactionManager::commit_transaction(&mut *self.0).await
    }

    async fn rollback(mut self) -> Result<(), diesel::result::Error> {
        AnsiTransactionManager::rollback_transaction(&mut *self.0).await
    }
}

#[derive(Debug)]
pub enum PgBackendError {
    Pool(String),
    Diesel(diesel::result::Error),
}

impl From<diesel::result::Error> for PgBackendError {
    fn from(e: diesel::result::Error) -> Self {
        PgBackendError::Diesel(e)
    }
}

pub struct PgBackend(Pool<AsyncPgConnection>);

impl PgBackend {
    pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
        Self(pool)
    }
}

#[async_trait]
impl Drive<PgContext> for PgBackend {
    type Error = PgBackendError;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, ScopedError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut PgContext) -> Result<T, E>
            + AsyncFnMark<&'c mut PgContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let mut conn = self
            .0
            .get()
            .await
            .map_err(|e| ScopedError::Backend(PgBackendError::Pool(e.to_string())))?;

        AnsiTransactionManager::begin_transaction(&mut *conn)
            .await
            .map_err(|e| ScopedError::Backend(PgBackendError::Diesel(e)))?;

        let mut context = PgContext(conn);

        let result = f(&mut context).await;

        match result {
            Ok(t) => {
                context
                    .commit()
                    .await
                    .map_err(|e| ScopedError::Backend(PgBackendError::Diesel(e)))?;
                Ok(t)
            }
            Err(e) => {
                let _ = context.rollback().await;
                Err(ScopedError::Advance(e))
            }
        }
    }
}

// ---- Advance implementations (ZSTs — no lifetime, no context field) ----

pub struct DecreaseProductAdvance;

#[async_trait]
impl Advance<DecreaseProduct, PgContext> for DecreaseProductAdvance {
    type Error = diesel::result::Error;

    async fn advance(
        &self,
        context: &mut PgContext,
        step: &DecreaseProduct,
    ) -> Result<(), diesel::result::Error> {
        diesel::sql_query("UPDATE products SET stock = stock - $1 WHERE id = $2")
            .bind::<diesel::sql_types::Integer, _>(step.quantity)
            .bind::<diesel::sql_types::Integer, _>(step.product_id)
            .execute(&mut *context.0)
            .await?;
        Ok(())
    }
}

pub struct CreateOrderAdvance;

#[async_trait]
impl Advance<CreateOrder, PgContext> for CreateOrderAdvance {
    type Error = diesel::result::Error;

    async fn advance(
        &self,
        context: &mut PgContext,
        step: &CreateOrder,
    ) -> Result<(), diesel::result::Error> {
        diesel::sql_query("INSERT INTO orders (user_id, product_id, quantity) VALUES ($1, $2, $3)")
            .bind::<diesel::sql_types::Integer, _>(step.user_id)
            .bind::<diesel::sql_types::Integer, _>(step.product_id)
            .bind::<diesel::sql_types::Integer, _>(step.quantity)
            .execute(&mut *context.0)
            .await?;
        Ok(())
    }
}

// ---- Entrypoint ----

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost:5432/test".into());

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder(config).build()?;

    let backend = PgBackend::new(pool);

    let result = run_order_usecase::<_, _, diesel::result::Error, _, _>(
        &backend,
        DecreaseProductAdvance,
        CreateOrderAdvance,
        1, // product_id
        1, // user_id
        1, // quantity
    )
    .await;

    match result {
        Ok(()) => println!("Transaction completed successfully"),
        Err(_) => eprintln!("Transaction failed"),
    }

    Ok(())
}
