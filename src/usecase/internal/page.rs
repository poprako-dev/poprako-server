use std::collections::HashMap;
use std::ops::Range;

use poprako_orchestra::{OperRun as _, Run};

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest, accept};

/// Pinned chapters and query coverage aligned with the caller's comic positions.
pub struct PinnedChapterSnapshot {
    /// Each loaded chapter is owned once.
    infos: Vec<ChapterInfo>,

    /// A queried position without a chapter is known absent.
    queried: Vec<bool>,
    /// Loaded chapter index for each caller position.
    positions: Vec<Option<usize>>,
}

impl PinnedChapterSnapshot {
    /// Loads a range of the supplied comic list without copying its identifiers.
    pub async fn load_from_comics<R>(
        repo: &R,
        comics: &[ComicInfo],
        range: Range<usize>,
    ) -> BaseRest<Self>
    where
        R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError> + Sync,
    {
        let mut queried = vec![false; comics.len()];

        let mut positions_by_id = HashMap::<_, Vec<_>>::new();

        for (position, comic) in comics.iter().enumerate() {
            //
            if range.contains(&position) {
                //
                if let Some(queried) = queried.get_mut(position) {
                    *queried = true;
                }

                positions_by_id
                    .entry(comic.id.as_str())
                    .or_default()
                    .push(position);
            }
        }

        let mut comic_ids = positions_by_id.keys().copied().collect::<Vec<_>>();

        comic_ids.sort_unstable();

        let infos = match comic_ids.as_slice() {
            //
            [] => Vec::new(),

            _ => {
                //
                ListPinnedChapterInfos {
                    comic_ids: &comic_ids,
                }
                .run_on(repo)
                .await?
            }
        };

        let mut positions = vec![None; comics.len()];

        for (info_position, info) in infos.iter().enumerate() {
            //
            if let Some(comic_positions) =
                positions_by_id.get(info.comic_id.as_str())
            {
                //
                for &comic_position in comic_positions {
                    //
                    if let Some(position) = positions.get_mut(comic_position) {
                        *position = Some(info_position);
                    }
                }
            }
        }

        accept(Self {
            infos,
            queried,
            positions,
        })
    }

    /// Returns the loaded chapters without rebuilding an owned-key map.
    pub fn infos(&self) -> &[ChapterInfo] {
        &self.infos
    }

    /// Borrows known present and absent results for cover fallback lookup.
    pub fn lookup<'a>(
        &'a self,
        comics: &'a [ComicInfo],
    ) -> HashMap<&'a str, Option<&'a ChapterInfo>> {
        //
        comics
            .iter()
            .enumerate()
            .filter_map(|(position, comic)| {
                //
                if !self.queried.get(position).copied().unwrap_or_default() {
                    return None;
                }

                let info = self
                    .positions
                    .get(position)
                    .copied()
                    .flatten()
                    .and_then(|position| self.infos.get(position));

                Some((comic.id.as_str(), info))
            })
            .collect()
    }

    /// Moves loaded chapters and their comic positions into response assembly.
    pub fn into_parts(self) -> (Vec<ChapterInfo>, Vec<Option<usize>>) {
        (self.infos, self.positions)
    }
}

/// First-page identifiers owned once, with positions aligned to requested comics.
pub struct ComicPageIds {
    /// Page identifiers moved from repository results.
    page_ids: Vec<String>,

    /// First-page position for every requested comic occurrence.
    positions: Vec<Option<usize>>,
}

impl ComicPageIds {
    /// Moves page identifiers and aligned positions into presentation hydration.
    pub fn into_parts(self) -> (Vec<String>, Vec<Option<usize>>) {
        (self.page_ids, self.positions)
    }
}

/// Loads page models needed by use-case orchestration.
pub struct PageLoader;

impl PageLoader {
    /// Resolves fallback pages using borrowed IDs and position-only associations.
    pub async fn load_ids_from_comics<R>(
        repo: &R,
        comic_ids: &[&str],
        pinned_chapters: Option<&HashMap<&str, Option<&ChapterInfo>>>,
    ) -> BaseRest<ComicPageIds>
    where
        R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
            + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
            + Sync,
    {
        let mut positions_by_comic_id = HashMap::<_, Vec<_>>::new();

        for (position, &comic_id) in comic_ids.iter().enumerate() {
            //
            positions_by_comic_id
                .entry(comic_id)
                .or_default()
                .push(position);
        }

        let mut chapter_comics = HashMap::new();

        let mut unloaded_ids = Vec::new();

        for &comic_id in positions_by_comic_id.keys() {
            //
            match pinned_chapters.and_then(|infos| infos.get(comic_id)) {
                //
                Some(Some(chapter)) => {
                    chapter_comics.insert(chapter.id.as_str(), comic_id);
                }

                Some(None) => {}

                None => unloaded_ids.push(comic_id),
            }
        }

        unloaded_ids.sort_unstable();

        let unloaded_chapters = match unloaded_ids.as_slice() {
            //
            [] => Vec::new(),

            _ => {
                //
                ListPinnedChapterInfos {
                    comic_ids: &unloaded_ids,
                }
                .run_on(repo)
                .await?
            }
        };

        for chapter in &unloaded_chapters {
            //
            chapter_comics
                .insert(chapter.id.as_str(), chapter.comic_id.as_str());
        }

        let mut page_ids = Vec::new();

        let mut positions = vec![None; comic_ids.len()];

        if chapter_comics.is_empty() {
            //
            return accept(ComicPageIds {
                page_ids,
                positions,
            });
        }

        let chapter_ids = chapter_comics.keys().copied().collect::<Vec<_>>();

        let pages = ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        }
        .run_on(repo)
        .await?;

        for page in pages {
            //
            let Some(comic_id) = chapter_comics.get(page.chapter_id.as_str())
            else {
                continue;
            };

            let Some(comic_positions) = positions_by_comic_id.get(comic_id)
            else {
                continue;
            };

            let page_position = page_ids.len();

            page_ids.push(page.id);

            for &comic_position in comic_positions {
                //
                if let Some(position) = positions.get_mut(comic_position) {
                    *position = Some(page_position);
                }
            }
        }

        accept(ComicPageIds {
            page_ids,
            positions,
        })
    }
}
