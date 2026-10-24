//! Process-local online-user lease cache adapter operations.

#[cfg(test)]
// Online-user adapter tests use explicit instants without sleeping.
mod tests;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use poprako_orchestra::Run;

use crate::config::cache::CacheConfig;
use crate::part::cache::oper::online_user::{
    ListOnlineUserIds, MarkUserOnline,
};
use crate::part_impl::cache::mem_impl::MemCache;
use crate::result::{BaseError, BaseRest};

/// Shared team-partitioned online-user leases.
#[derive(Clone)]
pub struct OnlineUserCache {
    /// Lease duration applied by every refresh.
    ttl: Duration,
    /// User expiration instants grouped by team.
    deadlines: Arc<DashMap<String, HashMap<String, Instant>>>,
}

impl OnlineUserCache {
    /// Builds an empty cache with the configured online lease duration.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            deadlines: Arc::default(),
        }
    }

    /// Refreshes one online-user deadline using an explicit clock instant.
    pub fn mark_user_online_at(
        &self,
        team_id: &str,
        user_id: &str,
        now: Instant,
    ) {
        //
        let Some(expires_at) = now.checked_add(self.ttl) else {
            return;
        };

        self.deadlines
            .entry(team_id.into())
            .or_default()
            .insert(user_id.into(), expires_at);
    }

    /// Lists one team's active users using an explicit clock instant.
    #[must_use]
    pub fn list_online_user_ids_at(
        &self,
        team_id: &str,
        now: Instant,
    ) -> Vec<String> {
        //
        let Entry::Occupied(mut team_entry) =
            self.deadlines.entry(team_id.into())
        else {
            return Vec::new();
        };

        let (mut online_user_ids, is_empty) = {
            //
            let online_user_deadlines = team_entry.get_mut();

            online_user_deadlines.retain(|_, expires_at| *expires_at > now);

            (
                online_user_deadlines.keys().cloned().collect::<Vec<_>>(),
                online_user_deadlines.is_empty(),
            )
        };

        if is_empty {
            team_entry.remove();
        }

        online_user_ids.sort();

        online_user_ids
    }
}

impl Default for OnlineUserCache {
    // Use the same lease policy as a cache adapter with default configuration.
    fn default() -> Self {
        Self::new(CacheConfig::default().online_user_ttl)
    }
}

impl Run<MarkUserOnline<'_>> for MemCache {
    // Keep memory adapter failures on the shared cache adapter error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Refresh one user's online lease in the target team.
    async fn run(&self, oper: &MarkUserOnline<'_>) -> BaseRest<()> {
        //
        self.online_users.mark_user_online_at(
            oper.team_id(),
            oper.user_id(),
            Instant::now(),
        );

        accept!(())
    }
}

impl Run<ListOnlineUserIds<'_>> for MemCache {
    // Keep memory adapter failures on the shared cache adapter error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Remove expired leases and return the target team's active user ids.
    async fn run(&self, oper: &ListOnlineUserIds<'_>) -> BaseRest<Vec<String>> {
        //
        let online_user_ids = self
            .online_users
            .list_online_user_ids_at(oper.team_id(), Instant::now());

        accept!(online_user_ids)
    }
}
