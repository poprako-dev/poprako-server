use crate::model::termbase::TermbaseEntry;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

/// Input parameters for creating a termbase, scoped to exactly one of team or comic.
pub struct CreateTermbaseParams {
    pub name: String,
    pub team_id: Option<String>,
    pub comic_id: Option<String>,
}

impl TryInto<TermbaseEntry> for CreateTermbaseParams {
    type Error = BaseError;

    fn try_into(self) -> BaseResult<TermbaseEntry> {
        //
        let termbase_id = next_snowflake_id();

        match (self.team_id, self.comic_id) {
            //
            (Some(team_id), None) => accept(TermbaseEntry::Team {
                id: termbase_id,
                name: self.name,
                team_id,
            }),

            (None, Some(comic_id)) => accept(TermbaseEntry::Comic {
                id: termbase_id,
                name: self.name,
                comic_id,
            }),

            _ => Err(invalid_termbase_scope_error()),
        }
    }
}

fn invalid_termbase_scope_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: "exactly one of team_id or comic_id must be provided".into(),
    }
}
