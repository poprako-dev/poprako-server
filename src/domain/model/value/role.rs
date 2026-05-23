use serde::{Deserialize, Serialize};

pub struct RoleMask(u32);

pub enum Role {
    RawProvider = 1 << 0,
    Translator = 1 << 1,
    Proofreader = 1 << 2,
    Typesetter = 1 << 3,
    Reviewer = 1 << 4,
    Publisher = 1 << 5,
    Admin = 1 << 6,
}
