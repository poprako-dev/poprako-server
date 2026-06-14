use crate::state::{StateAdvance, StateTransactional};
use crate::step::Step;

pub struct Proxy<S> {
    state: S,
    rollback: bool,
}

impl<S> Proxy<S> {
    pub fn new(state: S) -> Self {
        Self {
            state,
            rollback: false,
        }
    }

    // TODO: comment.
    pub async fn run<St>(&mut self, step: St) -> Result<St::Output, St::Error>
    where
        St: Step,
        S: StateAdvance<St>,
    {
        self.state
            .advance(step)
            .await
            .inspect_err(|_| self.rollback = true)
    }

    pub fn rollback(&self) -> bool {
        self.rollback
    }

    pub fn into_state(self) -> S {
        self.state
    }
}
