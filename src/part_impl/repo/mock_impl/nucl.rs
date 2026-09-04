use poprako_orchestra::Nucl;
use poprako_orchestra::nucl::Error as NuclError;

use crate::part::nucl::Serial;
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest, accept};

/// Applies a signed counter delta without allowing an invalid negative state.
pub fn apply_signed_delta(value: &mut usize, delta: i32) -> BaseRest<()> {
    //
    match delta.cmp(&0) {
        //
        std::cmp::Ordering::Greater => {
            //
            let delta = usize::try_from(delta).map_err(|_| {
                //
                BaseError::Unrecoverable {
                    message: "signed delta conversion failed".into(),
                }
            })?;

            *value = value.checked_add(delta).ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "counter overflow".into(),
                }
            })?;
        }

        //
        std::cmp::Ordering::Equal => {
            //
        }

        std::cmp::Ordering::Less => {
            //
            let delta =
                usize::try_from(delta.unsigned_abs()).map_err(|_| {
                    //
                    BaseError::Unrecoverable {
                        message: "signed delta conversion failed".into(),
                    }
                })?;

            *value = value.checked_sub(delta).ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "counter underflow".into(),
                }
            })?;
        }
    }

    //
    accept(())
}

impl Nucl for Mock {
    // Uses the strongest mock isolation marker for every transaction test.
    type Level = Serial;

    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
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
            obj_delete_failure: flags.obj_delete_failure,
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
