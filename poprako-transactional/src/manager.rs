pub mod result;

use crate::backend::Backend;
use crate::handle::Handle as _;
use crate::manager::result::Error as ScopedError;
use crate::util::DynFut;

pub struct Manager<B> {
    backend: B,
}

impl<B> Manager<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

impl<B> Manager<B>
where
    B: Backend,
{
    pub async fn transactional_scoped<S, W, T, E, F>(
        &self,
        func: F,
    ) -> Result<T, ScopedError<B::Error, E>>
    where
        F: for<'s> FnOnce(&'s mut B::Handle) -> DynFut<'s, Result<T, E>>,
    {
        // FIXME: cancellation drop.
        let mut handle = self.backend.begin().await.map_err(ScopedError::Begin)?;

        let result = func(&mut handle).await;

        let Ok(output) = result else {
            if let Err(re) = handle.rollback().await {
                return Err(ScopedError::Rollback(None, Some(re)));
            }
            return Err(ScopedError::StepError(result.err().unwrap()));
        };

        handle.commit().await.map_err(ScopedError::Commit)?;

        // FIXME: cancellation drop.

        Ok(output)
    }
}
