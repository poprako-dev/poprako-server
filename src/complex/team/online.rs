//! Process-local online-user leases for teams.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;

use crate::complex::team::TeamComplex;

// One successful mark keeps a user online in the target team for ten minutes.
const ONLINE_USER_TTL: Duration = Duration::from_secs(10 * 60);

// Online deadlines are partitioned by team so listing touches one team only.
static ONLINE_USER_DEADLINES: LazyLock<
    DashMap<String, HashMap<String, Instant>>,
> = LazyLock::new(DashMap::new);

#[cfg(test)]
// Online-user lease tests use explicit instants and unique team identifiers.
mod tests;

impl TeamComplex {
    /// Marks a user online in a team for the configured lease duration.
    pub fn mark_user_online(team_id: &str, user_id: &str) {
        mark_user_online_at(team_id, user_id, Instant::now());
    }

    /// Lists the active user identifiers for one team in ascending order.
    pub fn list_online_user_ids(team_id: &str) -> Vec<String> {
        list_online_user_ids_at(team_id, Instant::now())
    }
}

// Marks one user online using an explicit clock instant.
fn mark_user_online_at(team_id: &str, user_id: &str, now: Instant) {
    //
    let expires_at = now + ONLINE_USER_TTL;

    ONLINE_USER_DEADLINES
        .entry(team_id.into())
        .or_default()
        .insert(user_id.into(), expires_at);
}

// Lists one team's active users using an explicit clock instant.
fn list_online_user_ids_at(team_id: &str, now: Instant) -> Vec<String> {
    //
    let Entry::Occupied(mut team_entry) =
        ONLINE_USER_DEADLINES.entry(team_id.into())
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
