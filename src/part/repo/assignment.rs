//! Repository traits for the assignment domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::result::BaseError;

/// Assignment repository operations.
///
/// Read operations can run independently. Operations participating in an
/// atomic workflow step through the context supplied by `Nucl::coord`.
pub trait AssignmentRepo<C>:
    for<'a, 'b> Run<FindAssignmentInfo<'a, 'b>, Error = BaseError>
    + for<'a, 'b> Run<GetAssignmentInfo<'a, 'b>, Error = BaseError>
    + for<'a, 'b> Run<ListAssignmentInfos<'a, 'b>, Error = BaseError>
    + for<'a, 'b> Step<FindAssignmentInfo<'a, 'b>, C, Error = BaseError>
    + for<'a, 'b> Step<ListAssignmentInfos<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<ListAssignmentInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<CreateAssignment<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateAssignmentRoles<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteAssignments<'a>, C, Error = BaseError>
{
}

impl<T, C> AssignmentRepo<C> for T where
    T: for<'a, 'b> Run<FindAssignmentInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Run<GetAssignmentInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Run<ListAssignmentInfos<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Step<FindAssignmentInfo<'a, 'b>, C, Error = BaseError>
        + for<'a, 'b> Step<ListAssignmentInfos<'a, 'b>, C, Error = BaseError>
        + for<'a> Step<ListAssignmentInfosExcluded<'a>, C, Error = BaseError>
        + for<'a> Step<CreateAssignment<'a>, C, Error = BaseError>
        + for<'a> Step<UpdateAssignmentRoles<'a>, C, Error = BaseError>
        + for<'a> Step<DeleteAssignments<'a>, C, Error = BaseError>
{
}
