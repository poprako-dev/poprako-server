pub enum Error<E, BE> {
    /// An error occurred during the execution of a step.
    Advance(E),
    /// An error occurred during the execution of the backend.
    Backend(BE),
}
