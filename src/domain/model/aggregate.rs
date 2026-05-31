/// Zero-sized marker type that prevents struct literal construction
/// of input aggregates from outside the defining module.
///
/// Include `_m: PrivateMarker` as a field in any input aggregate struct
/// whose construction should be limited to `new()` constructors.
#[derive(Default, Clone, Copy)]
pub struct PrivateMarker;

impl std::fmt::Debug for PrivateMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FIXME: no output.
        write!(f, "")
    }
}

pub mod member;
pub mod member_invitation;
pub mod system_mail;
pub mod team;
pub mod user;
