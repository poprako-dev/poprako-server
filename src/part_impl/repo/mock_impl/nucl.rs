use poprako_orchestra::Nucl;
use poprako_orchestra::nucl::Error as NuclError;

use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::BaseError;

impl Nucl for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    // Internal type alias for `Context`.
    type Context = MockContext;

    // Internal implementation of `coord`.
    async fn coord<F, T, E>(&self, f: F) -> Result<T, NuclError<Self::Error, E>>
    where
        F: for<'cx> AsyncFnOnce(&'cx mut Self::Context) -> Result<T, E> + Send,
        T: Send,
        E: Send,
    {
        let state = self.state.lock().unwrap().clone();

        let flags = self.flags.lock().unwrap().clone();

        let mut context = MockContext {
            state,
            archive_commit_failure: flags.archive_commit_failure,
            create_team_failure: flags.create_team_failure,
        };

        match f(&mut context).await {
            //
            // Internal implementation detail.
            Ok(value) => {
                //
                // Internal implementation detail.
                *self.state.lock().unwrap() = context.state;

                Ok(value)
            }

            Err(error) => Err(NuclError::Step(error)),
        }
    }
}
