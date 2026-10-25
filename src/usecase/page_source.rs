//! source page image status, listing, allocation, and deletion.

/// source page-manifest allocation orchestration.
pub mod alloc;
/// source page deletion orchestration.
pub mod delete;
/// source page-list orchestration.
pub mod list;
/// source page presentation assembly.
pub mod view;

#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::key::ObjGen;
use poprako_obj_dept::oper::MarkObjUploaded;
use poprako_util::i18n::trl;

use crate::complex::chapter as chapter_complex;
use crate::complex::page_source::perm as page_source_perm_complex;
use crate::data::instr::page_source::MarkPageSourceImageUploadedInstr;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageSourceImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_source::{
    GetPageSourceInfo, GetPageSourceInfoExcluded,
};
use crate::part::repo::page_source::PageSourceRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::business_id::PageSourceId;

/// Optimistically marks the requested current page generation as uploaded.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
pub async fn mark_image_uploaded<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    id: String,
    instr: MarkPageSourceImageUploadedInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: PageSourceRepo<C>
        + AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    let page_source_info = GetPageSourceInfo { id: &id }.run_on(repo).await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &page_source_info.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        chapter_complex::ensure_chapter_writable(&chapter_info)?;

        GetPageSourceInfoExcluded { id: &id }
            .step_on(repo, context)
            .await?;

        let assignment_info = FindAssignmentInfo::ChapterUser {
            chapter_id: &chapter_info.id,
            user_id: &token.user_id,
        }
        .step_on(repo, context)
        .await?;

        let Some(assignment_info) = assignment_info else {
            //
            return Err(expected_error! {
                ExpectedVariant::Perm,
                message: trl("error-page-upload-role-required");
                "source image upload role required",
            });
        };

        page_source_perm_complex::ensure_user_can_mark_image_uploaded(
            &assignment_info,
        )?;

        // Confirmation is optimistic; the object worker reconciles physical presence.
        let image_key = ObjGen::<PageSourceImage>::new(
            PageSourceId::new(&id)?,
            instr.image_ver,
        );

        let marked = MarkObjUploaded::<PageSourceImage>::new(&image_key)
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

        if !marked {
            //
            return Err(expected_error! {
                ExpectedVariant::Args,
                message: trl("error-stale-page-image-upload");
                "source image upload generation is stale",
            });
        }

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept!(())
    })
    .await?;

    accept!(())
}
