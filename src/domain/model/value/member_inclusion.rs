// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// /// Typed value object for the `includes` query parameter on member endpoints.
// ///
// /// Parsed from a comma-separated string with valid tokens `user` and `team`.
// #[derive(Debug, Clone, Copy, Default)]
// pub struct MemberInclusion {
//     pub user: bool,
//     pub team: bool,
// }
// 
// impl MemberInclusion {
//     /// Parses a comma-separated includes string.
//     ///
//     /// Unknown tokens are silently ignored.
//     pub fn parse<I>(includes: &[I]) -> Self
//     where
//         I: AsRef<str>,
//     {
//         let user = includes.iter().any(|s| s.as_ref() == "user");
//         let team = includes.iter().any(|s| s.as_ref() == "team");
// 
//         Self { user, team }
//     }
// 
//     /// Returns true when no include flags are set.
//     pub fn is_empty(&self) -> bool {
//         !self.user && !self.team
//     }
// }
