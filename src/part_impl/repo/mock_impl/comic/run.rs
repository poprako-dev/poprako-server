use poprako_orchestra::Run;
use tracing::instrument;

use crate::model::read::proj::comic::ComicInfo;
use crate::part::repo::oper::comic::{
    GetComicInfo, ListComicInfos, UpdateComic,
};
use crate::part_impl::repo::mock_impl::comic::{
    get_comic_info, list_comic_infos,
};
use crate::part_impl::repo::mock_impl::{Mock, expected, now};
use crate::result::{BaseError, accept};

impl<'a, 'b> Run<GetComicInfo<'a, 'b>> for Mock {
    // Use base error type for get-by-id read operation.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load locked state and delegate to shared helper.
    async fn run(
        &self,
        oper: &GetComicInfo<'a, 'b>,
    ) -> Result<ComicInfo, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        get_comic_info(&state, oper.id, oper.incls)
    }
}

impl<'a> Run<ListComicInfos<'a>> for Mock {
    // Use base error type for list operation.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load locked state and execute listing helper.
    async fn run(
        &self,
        oper: &ListComicInfos<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        accept(list_comic_infos(&state, oper.spec))
    }
}

impl<'a> Run<UpdateComic<'a>> for Mock {
    // Use base error type for full-run metadata updates.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply mutable field updates and touch updated_at.
    async fn run(&self, oper: &UpdateComic<'a>) -> Result<(), Self::Error> {
        //
        let mut state = self.state.lock().unwrap();

        let comic = state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.update.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        comic.title = oper.update.title.clone();

        comic.author = oper.update.author.clone();

        comic.description = oper.update.description.clone();

        comic.updated_at = now();

        accept(())
    }
}
