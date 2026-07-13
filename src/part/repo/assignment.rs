//! Repository traits for the assignment domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::result::RegularError;

/// Assignment repository operations.
///
/// Read operations can run independently. Operations participating in an
/// atomic workflow step through the context supplied by `Nucl::coord`.
pub trait AssignmentRepo<C>:
    for<'a, 'b> Run<FindAssignmentInfo<'a, 'b>, Error = RegularError>
    + for<'a, 'b> Run<GetAssignmentInfo<'a, 'b>, Error = RegularError>
    + for<'a, 'b> Run<ListAssignmentInfos<'a, 'b>, Error = RegularError>
    + for<'a, 'b> Step<FindAssignmentInfo<'a, 'b>, C, Error = RegularError>
    + for<'a, 'b> Step<ListAssignmentInfos<'a, 'b>, C, Error = RegularError>
    + for<'a> Step<ListAssignmentInfosExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<CreateAssignment<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateAssignmentRoles<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteAssignments<'a>, C, Error = RegularError>
{
}
