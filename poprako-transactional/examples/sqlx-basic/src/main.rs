use async_trait::async_trait;
use poprako_transactional::advance::Advance;
use poprako_transactional::run::Run;
use poprako_transactional::run::result::Error as ScopedError;
use poprako_transactional::step::Step;
use poprako_transactional::util::AsyncFnMark;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;

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

async fn run_order_usecase<M, H, E, D, C>(
    backend: &M,
    mut decrease_adv: D,
    mut create_adv: C,
    product_id: i32,
    user_id: i32,
    quantity: i32,
) -> Result<(), ScopedError<E, M::Error>>
where
    M: Run<H>,
    E: Send,
    D: Advance<DecreaseProduct, H> + Send,
    C: Advance<CreateOrder, H> + Send,
    E: From<D::Error> + From<C::Error>,
    H: Send,
{
    backend
        .scope::<(), E, _>(async move |handle| {
            decrease_adv
                .advance(
                    DecreaseProduct {
                        product_id,
                        quantity,
                    },
                    handle,
                )
                .await?;

            create_adv
                .advance(
                    CreateOrder {
                        user_id,
                        product_id,
                        quantity,
                    },
                    handle,
                )
                .await?;

            Ok(())
        })
        .await
}

// ---- Infra: sqlx ----

pub struct PgHandle(Transaction<'static, Postgres>);

impl PgHandle {
    async fn commit(self) -> Result<(), sqlx::Error> {
        self.0.commit().await
    }

    async fn rollback(self) -> Result<(), sqlx::Error> {
        self.0.rollback().await
    }
}

pub struct PgBackend(PgPool);

impl PgBackend {
    pub fn new(pool: PgPool) -> Self {
        Self(pool)
    }
}

#[async_trait]
impl Run<PgHandle> for PgBackend {
    type Error = sqlx::Error;

    async fn scope<T, E, F>(&self, f: F) -> Result<T, ScopedError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'h> F: AsyncFnOnce(&'h mut PgHandle) -> Result<T, E>
            + AsyncFnMark<&'h mut PgHandle, Result<T, E>, Fut: Send>
            + Send,
    {
        let tx = self.0.begin().await.map_err(ScopedError::Backend)?;
        let mut handle = PgHandle(tx);

        let result = f(&mut handle).await;

        match result {
            Ok(t) => {
                handle.commit().await.map_err(ScopedError::Backend)?;
                Ok(t)
            }
            Err(e) => {
                let _ = handle.rollback().await;
                Err(ScopedError::Advance(e))
            }
        }
    }
}

// ---- Advance implementations (ZSTs — no lifetime, no handle field) ----

pub struct DecreaseProductAdvance;

#[async_trait]
impl Advance<DecreaseProduct, PgHandle> for DecreaseProductAdvance {
    type Error = sqlx::Error;

    async fn advance(
        &mut self,
        step: DecreaseProduct,
        handle: &mut PgHandle,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE products SET stock = stock - $1 WHERE id = $2")
            .bind(step.quantity)
            .bind(step.product_id)
            .execute(&mut *handle.0)
            .await?;
        Ok(())
    }
}

pub struct CreateOrderAdvance;

#[async_trait]
impl Advance<CreateOrder, PgHandle> for CreateOrderAdvance {
    type Error = sqlx::Error;

    async fn advance(
        &mut self,
        step: CreateOrder,
        handle: &mut PgHandle,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO orders (user_id, product_id, quantity) VALUES ($1, $2, $3)")
            .bind(step.user_id)
            .bind(step.product_id)
            .bind(step.quantity)
            .execute(&mut *handle.0)
            .await?;
        Ok(())
    }
}

// ---- Entrypoint ----

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost:5432/test".into());
    let pool = PgPool::connect(&database_url).await?;

    let backend = PgBackend::new(pool);

    let result = run_order_usecase::<_, _, sqlx::Error, _, _>(
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
