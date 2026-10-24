//! Domain models for user authentication and profile storage.

/// The data needed to insert a new user row.
#[cfg_attr(test, derive(Clone))]
pub struct UserEntry {
    /// Server-assigned unique user identifier.
    pub id: String,

    /// Third-party OAuth or QID provider identifier for this user.
    pub qid: String,
    /// Display name shown throughout the application.
    pub nickname: String,

    /// Bcrypt or similar hashed password for credential verification.
    pub password_hash: String,
}

impl UserEntry {
    /// Borrows normalized qid without copying its storage.
    pub fn trimmed_qid(&self) -> &str {
        self.qid.trim()
    }

    /// Borrows normalized nickname without copying its storage.
    pub fn trimmed_nickname(&self) -> &str {
        self.nickname.trim()
    }
}

/// Mutable user profile fields updated together.
pub struct UserInfoRepl {
    /// The user identifier.
    pub id: String,

    /// The OAuth qualified identifier.
    pub qid: String,
    /// The display nickname.
    pub nickname: String,
}

impl UserInfoRepl {
    /// Borrows normalized qid without copying its storage.
    pub fn trimmed_qid(&self) -> &str {
        self.qid.trim()
    }

    /// Borrows normalized nickname without copying its storage.
    pub fn trimmed_nickname(&self) -> &str {
        self.nickname.trim()
    }
}

/// A user's credential fields updated together.
pub struct UserCredsRepl {
    /// The user identifier.
    pub id: String,

    /// The new password hash.
    pub password_hash: String,
}
