use crate::model::termbase::TermbaseEntry;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::next_snowflake_id;

pub struct CreateTermbaseParams {
    pub name: String,
    pub team_id: Option<String>,
    pub comic_id: Option<String>,
}

impl TryInto<TermbaseEntry> for CreateTermbaseParams {
    type Error = RegularError;

    fn try_into(self) -> RegularResult<TermbaseEntry> {
        //
        let termbase_id = next_snowflake_id();

        match (self.team_id, self.comic_id) {
            //
            (Some(team_id), None) => Ok(TermbaseEntry::Team {
                id: termbase_id,
                name: self.name,
                team_id,
            }),

            (None, Some(comic_id)) => Ok(TermbaseEntry::Comic {
                id: termbase_id,
                name: self.name,
                comic_id,
            }),

            _ => Err(invalid_termbase_scope_error()),
        }
    }
}

fn invalid_termbase_scope_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: "exactly one of team_id or comic_id must be provided".into(),
    }
}
