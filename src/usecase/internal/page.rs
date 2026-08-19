use std::collections::HashMap;

use poprako_orchestra::{OperRun as _, Run};

use crate::model::read::proj::page::PageInfo;
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest};

/// Loads page models needed by use-case orchestration.
pub struct PageLoader;

impl PageLoader {
    /// Loads uploaded first pages keyed by their owning comic identifiers.
    pub async fn load_infos_from_comics<R>(
        repo: &R,
        comic_ids: &[String],
    ) -> BaseRest<HashMap<String, PageInfo>>
    where
        R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
            + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
            + Sync,
    {
        if comic_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let chapter_infos =
            ListPinnedChapterInfos { comic_ids }.run_on(repo).await?;

        let comic_ids_by_chapter_id = chapter_infos
            .iter()
            .map(|chapter_info| {
                (chapter_info.id.clone(), chapter_info.comic_id.clone())
            })
            .collect::<HashMap<_, _>>();

        let chapter_ids =
            comic_ids_by_chapter_id.keys().cloned().collect::<Vec<_>>();

        let page_infos = ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        }
        .run_on(repo)
        .await?;

        let page_infos = page_infos
            .into_iter()
            .filter(|page_info| page_info.is_image_uploaded == Some(true))
            .filter_map(|page_info| {
                //
                comic_ids_by_chapter_id
                    .get(&page_info.chapter_id)
                    .cloned()
                    .map(|comic_id| (comic_id, page_info))
            })
            .collect();

        Ok(page_infos)
    }
}
