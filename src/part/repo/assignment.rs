//! Repository traits for the assignment domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::result::BaseError;

/// Assignment repository operations.
///
/// Read operations can run independently. Operations participating in an
/// atomic workflow step through the context supplied by `Nucl::coord`.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a, 'b> FindAssignmentInfo<'a, 'b>,
        for<'a, 'b> GetAssignmentInfo<'a, 'b>,
        for<'a, 'b> ListAssignmentInfos<'a, 'b>,
    ),
    step(
        for<'a, 'b> FindAssignmentInfo<'a, 'b>,
        for<'a, 'b> ListAssignmentInfos<'a, 'b>,
        for<'a> ListAssignmentInfosExcluded<'a>,
        for<'a> CreateAssignment<'a>,
        for<'a> UpdateAssignmentRoles<'a>,
        for<'a> DeleteAssignments<'a>,
    ),
)]
pub trait AssignmentRepo<C> {}
