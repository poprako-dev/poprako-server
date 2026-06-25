use uuid::Uuid;

use crate::complex::comic::ComicComplex;
use crate::part::prom::PromTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::result::RootResult;

pub struct WorksetComplex;

impl WorksetComplex {
    pub fn gen_id() -> String {
        format!("workset-{}", Uuid::now_v7())
    }

    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: WorksetRepoTransactional<C> + ComicRepoTransactional<C> + Send + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let _workset_info = repo
            .advance(context, &WorksetStep::get_info_excluded(id))
            .await?;

        let comic_infos = repo
            .advance(context, &ComicStep::list_by_workset_id_excluded(id))
            .await?;

        for comic_info in comic_infos {
            ComicComplex::delete_cascade(repo, prom, context, &comic_info.id).await?;
        }

        repo.advance(context, &WorksetStep::delete(id)).await?;

        Ok(())
    }
}
