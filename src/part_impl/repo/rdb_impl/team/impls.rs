use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::team::TeamInfo;
use crate::model::write::team::TeamAvatarReservation;
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfoExcluded,
    LockTeam, ReserveTeamAvatar, UpdateTeam,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::team::helpers::{
    create, delete, get_info_excluded, increment_workset_next_index, lock_team,
    mark_avatar_uploaded, reserve_avatar, update_info,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<UpdateTeam<'_>> for HybRepo {
    // Keep update orchestration failures compatible with other team operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Route team mutation variants into the corresponding SQL update handlers.
    async fn run(&self, oper: &UpdateTeam<'_>) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateTeam::Info { repl } => {
                submit_query!(self.core, update_info, repl)
            }

            UpdateTeam::MarkAvatarUploaded { repl } => {
                //
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    &repl.id,
                    repl.avatar_version,
                    repl.avatar_key.as_deref(),
                    repl.is_avatar_uploaded
                )
            }
        }
    }
}

impl<L> Step<CreateTeam<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Convert repository step failures to base error during transaction execution.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist a new team row within an open transaction and return persisted info.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateTeam<'_>,
    ) -> BaseRest<TeamInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<UpdateTeam<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Keep transactional team updates on the same base error contract.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply either profile updates or avatar flag updates in current transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateTeam<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateTeam::Info { repl } => {
                update_info(context.conn(), repl).await
            }

            UpdateTeam::MarkAvatarUploaded { repl } => {
                //
                mark_avatar_uploaded(
                    context.conn(),
                    &repl.id,
                    repl.avatar_version,
                    repl.avatar_key.as_deref(),
                    repl.is_avatar_uploaded,
                )
                .await
            }
        }
    }
}

impl<L> Step<ReserveTeamAvatar<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Report avatar-reservation validation and mutation errors through base error.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Reserve the next avatar slot and return upload reservation metadata.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ReserveTeamAvatar<'_>,
    ) -> BaseRest<TeamAvatarReservation> {
        //
        reserve_avatar(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl<L> Step<GetTeamInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Preserve consistent error typing for locked team detail fetches.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load team info with row lock and exclusion rules for transactional safety.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetTeamInfoExcluded<'_>,
    ) -> BaseRest<TeamInfo> {
        //
        match oper {
            //
            GetTeamInfoExcluded::Id { id } => {
                get_info_excluded(context.conn(), id).await
            }
        }
    }
}

impl<L> Step<LockTeam<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Keep lock contention errors on the shared repository error type.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Acquire row lock for update sequencing before sensitive team writes.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &LockTeam<'_>,
    ) -> BaseRest<()> {
        lock_team(context.conn(), oper.id).await
    }
}

impl<L> Step<DeleteTeam<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use the common base error for hard delete operations in transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Remove a team row after the caller has coordinated any dependent effects.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteTeam<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<L> Step<AllocTeamWorksetIndex<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Keep index allocation failures mapped to repository base errors.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Atomically increment and return previous index for next workset reservation.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &AllocTeamWorksetIndex<'_>,
    ) -> BaseRest<i32> {
        increment_workset_next_index(context.conn(), oper.id).await
    }
}
