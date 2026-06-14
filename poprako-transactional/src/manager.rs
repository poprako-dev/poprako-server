pub mod result;

use crate::manager::result::Error as ScopedError;
use crate::proxy::Proxy;
use crate::state::{Backend, StateTransactional};
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
    pub async fn transactional_scoped<S, I, T, E, F>(
        &self,
        init: I,
        func: F,
    ) -> Result<T, ScopedError<B::Error, S::Error, E>>
    where
        S: StateTransactional,
        I: FnOnce(B::Handle) -> S,
        F: for<'s> FnOnce(&'s mut Proxy<S>) -> DynFut<'s, Result<T, E>>,
    {
        let handle = self.backend.begin().await.map_err(ScopedError::Begin)?;

        let state = init(handle);

        let mut proxy = Proxy::new(state);

        let result = func(&mut proxy).await;

        let rollback = proxy.rollback();
        let state = proxy.into_state();

        let output = match result {
            Ok(o) => o,
            Err(e) => {
                if let Err(re) = state.rollback().await {
                    return Err(ScopedError::Rollback(Some(e), Some(re)));
                }
                return Err(ScopedError::StepError(e));
            }
        };

        if rollback {
            if let Err(re) = state.rollback().await {
                return Err(ScopedError::Rollback(None, Some(re)));
            }
            return Err(ScopedError::Rollback(None, None));
        }

        state.commit().await.map_err(ScopedError::Commit)?;

        Ok(output)
    }
}
