use poprako_orchestra::Context;

/// Selects whether a loader executes independently or in a caller transaction.
pub enum LoadMode<'a, C>
where
    C: Context,
{
    /// Executes the operation independently.
    Run,

    /// Executes the operation through the caller-owned transaction context.
    Step {
        /// The active transaction context.
        context: &'a mut C,
    },
}
