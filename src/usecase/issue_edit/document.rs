//! Consistent document assembly within the caller's transaction.

use poprako_orchestra::{Context, OperStep as _};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::oper::ListObjMetas;

use crate::complex::issue_edit as issue_edit_complex;
use crate::data::val::issue_edit::{IssueEditPageVal, IssueEditVariantsVal};
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::issue_edit::IssueEditRepo;
use crate::part::repo::oper::issue_edit::{
    GetIssuePageRevision, GetReviewVer, ListIssueVariantCounts,
    ListPageIssueInfos,
};
use crate::result::{BaseError, BaseRest};

/// Assembles a consistent page document.
pub async fn page<C, R, O>(
    repo: &R,
    obj_dept: &O,
    context: &mut C,
    chapter_id: &str,
    page_id: &str,
) -> BaseRest<IssueEditPageVal>
where
    C: Context + Send,
    R: IssueEditRepo<C> + Sync,
    O: ObjDeptView<PageArtworkImage, C> + Sync,
{
    let ver = GetReviewVer { chapter_id }.step_on(repo, context).await?;

    let revision = GetIssuePageRevision {
        page_artwork_id: page_id,
    }
    .step_on(repo, context)
    .await?;

    let ids = [page_id];

    let images = ListObjMetas::<PageArtworkImage>::new(&ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let issues = ListPageIssueInfos {
        page_artwork_id: page_id,
    }
    .step_on(repo, context)
    .await?;

    accept!(IssueEditPageVal {
        page_artwork_id: page_id.to_owned(),
        review_key: issue_edit_complex::review_key(chapter_id, ver),
        revision: revision.to_string(),
        chapter_revision: ver.revision.to_string(),
        image_ver: images.get(page_id).map(|meta| meta.key.ver),
        issues: issues.into_iter().map(Into::into).collect(),
    })
}

/// Assembles chapter category counts.
pub async fn variants<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
) -> BaseRest<IssueEditVariantsVal>
where
    C: Context + Send,
    R: IssueEditRepo<C> + Sync,
{
    let ver = GetReviewVer { chapter_id }.step_on(repo, context).await?;

    let counts = ListIssueVariantCounts { chapter_id }
        .step_on(repo, context)
        .await?;

    accept!(IssueEditVariantsVal {
        review_key: issue_edit_complex::review_key(chapter_id, ver),
        revision: ver.revision.to_string(),
        variants: counts.into_iter().map(Into::into).collect(),
    })
}
