//! Page image status, listing, reservation, and deletion.

/// Page deletion orchestration.
pub mod delete;
/// Page-list orchestration.
pub mod list;
/// Page-manifest reservation orchestration.
pub mod reserve;
/// Page presentation assembly.
pub mod view;

#[cfg(test)]
mod tests;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::oper::MarkObjUploadedOutcome;
use poprako_obj_dept::{ObjDept, obj_inst};
use poprako_util::i18n::trl;

use crate::complex::page::PagePermComplex;
use crate::data::instr::page::MarkPageImageUploadedInstr;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::PageImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::page::GetPageInfo;
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Optimistically marks the requested current page generation as uploaded.
#[instrument(level = "info", skip(repo, obj_dept))]
pub async fn mark_image_uploaded<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
    instr: MarkPageImageUploadedInstr,
) -> BaseRest<()>
where
    C: Context,
    R: PageRepo<C> + AssignmentRepo<C>,
    O: ObjDept<PageImage, C> + Sync,
{
    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &page_info.chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-page-upload-role-required"),
        });
    };

    PagePermComplex::ensure_user_can_mark_image_uploaded(&assignment_info)?;

    // SAFETY: This is an optimistic exact-generation transition. It does not
    // synchronously prove PUT success, object presence, or content integrity;
    // the delayed actor may reset this generation after a failed HEAD check.
    let image_key = ObjKey {
        id,
        version: instr.image_version,
    };

    let marked = obj_inst! { MarkObjUploaded<PageImage> { key: &image_key } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    match marked {
        //
        MarkObjUploadedOutcome::Marked => accept(()),

        MarkObjUploadedOutcome::NotCurrent => Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-page-image-upload"),
        }),
    }
}
