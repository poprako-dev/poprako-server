//! Team ownership projection queries.

use diesel::{ExpressionMethods as _, OptionalExtension as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::part::repo::oper::team::ResolveTeamId;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::{t_chapter, t_comic, t_workset};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

// Build the client-visible error for an unresolvable comic or chapter.
fn missing_resource(oper: &ResolveTeamId<'_>) -> BaseError {
    //
    let (message, resource_kind, resource_id) = match oper {
        //
        ResolveTeamId::Comic { id } => {
            (trl("error-comic-not-found"), "comic", *id)
        }

        ResolveTeamId::Chapter { id } => {
            (trl("error-chapter-not-found"), "chapter", *id)
        }
    };

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %message,
        resource_kind = %resource_kind,
        resource_id = %resource_id,
        operation = "resolve team id",
        "expected ownership resolution error",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message,
    }
}

// Resolve the owning team id for a comic or chapter in a single query.
#[instrument(level = "info", skip_all)]
async fn resolve_team_id(
    conn: &mut RdbConn,
    oper: &ResolveTeamId<'_>,
) -> BaseRest<String> {
    //
    let team_id = match oper {
        //
        ResolveTeamId::Comic { id } => t_comic::table
            .inner_join(t_workset::table)
            .filter(t_comic::f_id.eq(id))
            .select(t_workset::f_team_id)
            .get_result(conn)
            .await
            .optional()
            .map_err(diesel)?,

        ResolveTeamId::Chapter { id } => t_chapter::table
            .inner_join(t_comic::table.inner_join(t_workset::table))
            .filter(t_chapter::f_id.eq(id))
            .select(t_workset::f_team_id)
            .get_result(conn)
            .await
            .optional()
            .map_err(diesel)?,
    };

    let Some(team_id) = team_id else {
        return Err(missing_resource(oper));
    };

    accept(team_id)
}

impl Run<ResolveTeamId<'_>> for HybRepo {
    // BaseError for the standalone team-resolution projection.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Executes the team-resolution projection on a pooled connection.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ResolveTeamId<'_>,
    ) -> Result<String, Self::Error> {
        submit_query!(self.core, resolve_team_id, oper)
    }
}

impl<L> Step<ResolveTeamId<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // BaseError for the transactional team-resolution projection.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Executes the team-resolution projection inside the active transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ResolveTeamId<'_>,
    ) -> Result<String, Self::Error> {
        resolve_team_id(context.conn(), oper).await
    }
}
