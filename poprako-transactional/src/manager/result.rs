pub enum Error<BE, SE, E> {
    /// An error occurred during the compensation of a step.
    Begin(BE),
    /// An error occurred during the execution of a step.
    StepError(E),
    /// An error occurred during the rollback of whole transaction.
    Rollback(Option<E>, Option<SE>),
    /// An error occurred during the commit of whole transaction.
    Commit(SE),
}
