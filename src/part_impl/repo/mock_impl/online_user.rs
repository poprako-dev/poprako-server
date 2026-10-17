//! Mock online-user repository operations.

use std::time::Instant;

use poprako_orchestra::Run;

use crate::part::repo::oper::online_user::{ListOnlineUserIds, MarkUserOnline};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, BaseRest, accept};

impl Run<MarkUserOnline<'_>> for Mock {
    // Keep mock failures on the shared repository error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Refresh one user's online lease in the mock repository.
    async fn run(&self, oper: &MarkUserOnline<'_>) -> BaseRest<()> {
        //
        self.online_users.mark_user_online_at(
            oper.team_id,
            oper.user_id,
            Instant::now(),
        );

        accept(())
    }
}

impl Run<ListOnlineUserIds<'_>> for Mock {
    // Keep mock failures on the shared repository error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Remove expired mock leases and return sorted active user ids.
    async fn run(&self, oper: &ListOnlineUserIds<'_>) -> BaseRest<Vec<String>> {
        //
        let online_user_ids = self
            .online_users
            .list_online_user_ids_at(oper.team_id, Instant::now());

        accept(online_user_ids)
    }
}
