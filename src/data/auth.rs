//! Data transfer objects for authentication use cases.

/// Input parameters for user registration.
pub struct RegisterData {
    pub qid: String,
    pub nickname: String,
    pub password: String,
    pub code: String,
}

/// Return value from a successful registration.
pub struct RegisterVal {
    pub user_id: String,
    pub token: String,
}

/// Input parameters for user login.
pub struct LoginData {
    pub qid: String,
    pub password: String,
}

/// Return value from a successful login.
pub struct LoginVal {
    pub user_id: String,
    pub token: String,
}
