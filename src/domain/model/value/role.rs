use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RoleMask(u32);
