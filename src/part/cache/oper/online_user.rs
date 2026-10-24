use poprako_orchestra::Oper;

/// Refreshes one user's online lease in a team.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkUserOnline<'a> {
    /// The team identifier.
    team_id: &'a str,

    /// The user identifier.
    user_id: &'a str,
}

impl<'a> MarkUserOnline<'a> {
    /// Selects the user's team-scoped lease to renew.
    #[must_use]
    pub const fn new(team_id: &'a str, user_id: &'a str) -> Self {
        Self { team_id, user_id }
    }

    /// Returns the team whose lease is renewed.
    #[must_use]
    pub const fn team_id(&self) -> &'a str {
        self.team_id
    }

    /// Returns the user whose lease is renewed.
    #[must_use]
    pub const fn user_id(&self) -> &'a str {
        self.user_id
    }
}

/// Lists active user identifiers for one team.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct ListOnlineUserIds<'a> {
    /// The team identifier.
    team_id: &'a str,
}

impl<'a> ListOnlineUserIds<'a> {
    /// Selects the team whose active leases are listed.
    #[must_use]
    pub const fn new(team_id: &'a str) -> Self {
        Self { team_id }
    }

    /// Returns the team whose active leases are listed.
    #[must_use]
    pub const fn team_id(&self) -> &'a str {
        self.team_id
    }
}
