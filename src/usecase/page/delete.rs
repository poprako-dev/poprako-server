use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;

use crate::complex::page::PagePermComplex;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageImage;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Deletes all pages under one chapter.
#[instrument(level = "info", skip(nucl, repo, obj_dept))]
pub async fn delete<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &chapter_id,
    )
    .await?;

    PagePermComplex::ensure_user_can_delete(&member_info)?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        let page_infos = ListPageInfos {
            chapter_id: &chapter_info.id,
        }
        .step_on(repo, context)
        .await?;

        let page_ids = page_infos
            .into_iter()
            .map(|page_info| page_info.id)
            .collect::<Vec<_>>();

        DeleteObjs::<PageImage>::new(&page_ids)
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

        DeletePages::Chapter {
            chapter_id: &chapter_info.id,
        }
        .step_on(repo, context)
        .await?;

        SetChapterPageCounters {
            id: &chapter_info.id,
            page_count: 0,
            total_unit_count: 0,
            translated_unit_count: 0,
            proofread_unit_count: 0,
        }
        .step_on(repo, context)
        .await?;

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
