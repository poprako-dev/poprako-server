/// Typed value object for the `includes` query parameter on member endpoints.
///
/// Parsed from a comma-separated string with valid tokens `user` and `team`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemberInclusion {
    pub user: bool,
    pub team: bool,
}

impl MemberInclusion {
    /// Parses a comma-separated includes string.
    ///
    /// Unknown tokens are silently ignored.
    pub fn parse(includes: Option<&str>) -> Self {
        let Some(includes_str) = includes else {
            return Self::default();
        };

        let mut user = false;
        let mut team = false;

        for part in includes_str.split(',') {
            match part.trim() {
                "user" => user = true,
                "team" => team = true,
                _ => {}
            }
        }

        Self { user, team }
    }

    /// Returns true when no include flags are set.
    pub fn is_empty(&self) -> bool {
        !self.user && !self.team
    }
}
