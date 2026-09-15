/// Fixed consumption queues shared by deferred tasks.
/// Payload variants select a queue independently of their operation names.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    //
    /// Serial queue for chapter work.
    Chapter,

    /// Serial queue shared by invitation operations.
    Invitation,
}

impl Topic {
    /// Returns the persisted name of this queue.
    pub const fn as_str(self) -> &'static str {
        //
        match self {
            //
            Self::Chapter => "chapter",

            Self::Invitation => "invitation",
        }
    }
}
