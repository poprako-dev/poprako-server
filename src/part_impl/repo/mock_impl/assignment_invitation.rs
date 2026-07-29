//! Mock implementations of assignment invitation repository opers.

use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part_impl::repo::mock_impl::{Mock, MockContext};

mod orchestra;

impl AssignmentInvitationRepo<MockContext> for Mock {}
