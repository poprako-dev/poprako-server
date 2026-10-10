//! Typed persistence for independent composite pages.

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
mod tests;

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::complex::page_artwork as page_artwork_complex;
use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page_artwork::{
    GetPageArtworkInfo, ListPageArtworkInfos, ReplacePageArtworkManifest,
    UpdatePageArtworkInfo,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::page_artwork::PageArtworkRow;
use crate::part_impl::repo::rdb_impl::schema::t_page_artwork;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel as map_diesel;

// Read stable page identities in manifest order.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<PageArtworkInfo>> {
    //
    let rows = t_page_artwork::table
        .filter(t_page_artwork::f_chapter_id.eq(chapter_id))
        .order(t_page_artwork::f_index.asc())
        .select(PageArtworkRow::as_select())
        .load::<PageArtworkRow>(conn)
        .await
        .map_err(map_diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

impl Run<GetPageArtworkInfo<'_>> for HybRepo {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes an independent composite page read.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetPageArtworkInfo<'_>,
    ) -> BaseRest<PageArtworkInfo> {
        //
        let mut conn = self.rdb_core.get().await?;

        let row = t_page_artwork::table
            .find(oper.id)
            .select(PageArtworkRow::as_select())
            .first::<PageArtworkRow>(&mut *conn)
            .await
            .optional()
            .map_err(map_diesel)?
            .ok_or_else(|| {
                //
                page_artwork_complex::error(
                    ExpectedVariant::Args,
                    "error-page-artwork-not-found",
                )
            })?;

        row.try_into()
    }
}

impl Run<ListPageArtworkInfos<'_>> for HybRepo {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes an independent composite page read.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListPageArtworkInfos<'_>,
    ) -> BaseRest<Vec<PageArtworkInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.chapter_id)
    }
}

impl<L> Step<ListPageArtworkInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes composite persistence in the caller transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageArtworkInfos<'_>,
    ) -> BaseRest<Vec<PageArtworkInfo>> {
        list_infos(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ReplacePageArtworkManifest<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes composite persistence in the caller transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ReplacePageArtworkManifest<'_>,
    ) -> BaseRest<()> {
        //
        let conn = context.conn();

        let retained_ids = oper
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>();

        diesel::delete(
            t_page_artwork::table
                .filter(t_page_artwork::f_chapter_id.eq(oper.chapter_id))
                .filter(t_page_artwork::f_id.ne_all(&retained_ids)),
        )
        .execute(conn)
        .await
        .map_err(map_diesel)?;

        diesel::update(
            t_page_artwork::table
                .filter(t_page_artwork::f_chapter_id.eq(oper.chapter_id)),
        )
        .set(t_page_artwork::f_index.eq(t_page_artwork::f_index * -1 - 1))
        .execute(conn)
        .await
        .map_err(map_diesel)?;

        for entries in oper.entries.chunks(2000) {
            //
            let rows = entries
                .iter()
                .map(PageArtworkRow::try_from)
                .collect::<BaseRest<Vec<_>>>()?;

            let upsert = diesel::insert_into(t_page_artwork::table)
                .values(&rows)
                .on_conflict(t_page_artwork::f_id)
                .do_update()
                .set((
                    t_page_artwork::f_index
                        .eq(excluded(t_page_artwork::f_index)),
                    t_page_artwork::f_raw_ident
                        .eq(excluded(t_page_artwork::f_raw_ident)),
                    t_page_artwork::f_updated_at
                        .eq(excluded(t_page_artwork::f_updated_at)),
                ));

            let count = diesel::query_dsl::methods::FilterDsl::filter(
                upsert,
                t_page_artwork::f_chapter_id
                    .eq(excluded(t_page_artwork::f_chapter_id)),
            )
            .execute(conn)
            .await
            .map_err(map_diesel)?;

            if count != entries.len() {
                //
                tracing::error!("composite manifest identity conflict");

                return Err(BaseError::Unrecoverable {
                    msg: "composite manifest identity conflict".into(),
                });
            }
        }

        accept(())
    }
}

impl<L> Step<UpdatePageArtworkInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Updates one page without rewriting the Chapter manifest.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdatePageArtworkInfo<'_>,
    ) -> BaseRest<()> {
        //
        let changed_count =
            diesel::update(t_page_artwork::table.find(&oper.update.id))
                .set((
                    t_page_artwork::f_raw_ident.eq(&oper.update.raw_ident),
                    t_page_artwork::f_updated_at.eq(OffsetDateTime::now_utc()),
                ))
                .execute(context.conn())
                .await
                .map_err(map_diesel)?;

        if changed_count != 1 {
            //
            return Err(page_artwork_complex::error(
                ExpectedVariant::Args,
                "error-page-artwork-not-found",
            ));
        }

        accept(())
    }
}
