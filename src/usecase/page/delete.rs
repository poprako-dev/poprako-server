use poprako_orchestra::{Nucl, OperProxy as _, OperStep as _, run_proxy};
use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use crate::complex::image::ImageComplex;
use crate::complex::page::PagePermComplex;
use crate::model::shared::user::UserToken;
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, accept};

/// Deletes all pages under one chapter.
#[instrument(level = "info", skip(nucl, repo, prom))]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    PagePermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

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

        let (mut delete_ids, mut delete_payloads) = (Vec::new(), Vec::new());

        for page_info in page_infos {
            //
            if let Some(object_key) = page_info.image_key {
                //
                delete_ids.push(ImageComplex::gen_delete_id());

                delete_payloads.push(TaskPayload::Image(
                    image::ImagePayload::Delete { object_key },
                ));
            }
        }

        let delete_tasks = delete_ids
            .iter()
            .zip(delete_payloads.iter())
            .map(|(id, payload)| Task {
                id,
                payload,
                delay: None,
            })
            .collect::<Vec<Task<'_, String, TaskPayload>>>();

        DeferBatch::new(&delete_tasks)
            .step_on(prom, context)
            .await?;

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
