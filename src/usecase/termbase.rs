use poprako_orchestra::{Nucl, run_proxy, step_proxy};
use tracing::instrument;

use crate::complex::termbase::{TermbaseComplex, TermbasePermComplex};
use crate::data::termbase::CreateTermbaseParams;
use crate::model::termbase::TermbaseEntry;
use crate::model::user::UserToken;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, LockTermbaseExcluded,
};
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::result::{BaseError, BaseResult, accept};

/// Creates a new termbase scoped to a team or comic.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create_termbase<C, R>(
    repo: &R,
    token: UserToken,
    params: CreateTermbaseParams,
) -> BaseResult<()>
where
    R: TermbaseRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    let entry = params.try_into()?;

    match &entry {
        //
        TermbaseEntry::Team { team_id, .. } => {
            TermbasePermComplex::ensure_user_can_create_team_termbase(
                &mut run_proxy! {
                    repo => for<'a> FindMemberInfo<'a>;
                },
                &token.user_id,
                team_id,
            )
            .await?;
        }

        TermbaseEntry::Comic { comic_id, .. } => {
            TermbasePermComplex::ensure_user_can_create_comic_termbase(
                &mut run_proxy! {
                    repo =>
                        for<'a, 'b> FindAssignmentInfo<'a, 'b>,
                        for<'a> FindMemberInfo<'a>;
                },
                &token.user_id,
                comic_id,
            )
            .await?;
        }
    }

    repo.run(&CreateTermbase { entry: &entry }).await?;

    accept(())
}

/// Deletes a termbase by id with cascade to its child terms.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete_termbase<C, N, R>(
    nucl: &N,
    repo: &R,
    _token: UserToken,
    id: String,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    R: TermbaseRepo<C> + TermRepo<C> + Sync,
{
    nucl.coord(async move |context| {
        //
        TermbaseComplex::delete_cascade(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a> LockTermbaseExcluded<'a>,
                    for<'v, 'a> DeleteTerms<'v, 'a>,
                    for<'a> DeleteTermbase<'a>;
            },
            &id,
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
