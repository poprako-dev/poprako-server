use async_trait::async_trait;

use poprako_transactional::step::Step;

/// Executes a single non-transactional [`Step`] against the repository.
/// It is designed to keep consistency with `Advance`.
///
/// Each `execute` call uses its own database connection and commits
/// independently. For opers that must be atomic with other steps,
/// use the transactional [`Advance`] path instead.
///
/// [`Advance`]: poprako_transactional::advance::Advance
#[async_trait]
pub trait Execute<S>
where
    S: Step,
{
    type Error;

    async fn execute(&self, step: &S) -> Result<S::Output, Self::Error>;
}
