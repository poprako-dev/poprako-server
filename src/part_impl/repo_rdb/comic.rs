//! RDB-backed comic repository — [`Execute`] and [`Advance`] implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::comic::{ComicCoverReservation, ComicInfo};
use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrChapterNextIndex, ListInfos,
    ListInfosExcluded, MarkCompleted, MarkCoverUploaded, ReserveCover, TouchLastActive,
    UpdateChapterCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::comic::{ComicAspect, ComicEntry, ComicRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::RegularError;

// ── Non-transactional ──────────────────────────────────────────────────────

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<ComicInfo, RegularError> {
        let mut conn = self.conn().await?;

        let row = schema::t_comic::table
            .filter(schema::t_comic::f_id.eq(step.id))
            .select(ComicRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-comic-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<ComicInfo>, RegularError> {
        let mut conn = self.conn().await?;

        let mut query = schema::t_comic::table
            .filter(schema::t_comic::f_workset_id.eq(step.spec.workset_id.as_str()))
            .select(ComicRow::as_select())
            .into_boxed();

        match &step.spec.fuzzy_title {
            Some(ft) => {
                query = query.filter(schema::t_comic::f_title.ilike(format!("%{}%", ft)));
            }
            None => {}
        }

        match step.spec.is_completed {
            Some(completed) => {
                query = query.filter(schema::t_comic::f_is_completed.eq(completed));
            }
            None => {}
        }

        let rows: Vec<ComicRow> = query
            .order_by(schema::t_comic::f_index.asc())
            .offset(step.spec.offset as i64)
            .limit(step.spec.limit as i64)
            .load(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;
        let now = OffsetDateTime::now_utc();

        let aspect = ComicAspect::new(now)
            .title(&step.update.title)
            .author(&step.update.author)
            .description(step.update.description.as_deref());

        diesel::update(
            schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.update.id.as_str())),
        )
        .set(&aspect)
        .execute(conn.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<MarkCoverUploaded<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &MarkCoverUploaded<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;
        let now = OffsetDateTime::now_utc();

        let affected = diesel::update(
            schema::t_comic::table
                .filter(schema::t_comic::f_id.eq(step.id))
                .filter(schema::t_comic::f_cover_version.eq(step.cover_version)),
        )
        .set((
            schema::t_comic::f_cover_uploaded.eq(true),
            schema::t_comic::f_updated_at.eq(now),
        ))
        .execute(conn.conn())
        .await
        .map_err(diesel)?;

        if affected == 0 {
            return Err(expected("error-cover-version-mismatch"));
        }

        Ok(())
    }
}

// ── Transactional ──────────────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> Result<ComicInfo, RegularError> {
        let entry = ComicEntry::from(step.form);

        let row = diesel::insert_into(schema::t_comic::table)
            .values(&entry)
            .returning(ComicRow::as_returning())
            .get_result(context.conn())
            .await
            .map_err(diesel)?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> Result<ComicInfo, RegularError> {
        let row = schema::t_comic::table
            .filter(schema::t_comic::f_id.eq(step.id))
            .select(ComicRow::as_select())
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-comic-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<ComicInfo, RegularError> {
        let row = schema::t_comic::table
            .filter(schema::t_comic::f_id.eq(step.id))
            .select(ComicRow::as_select())
            .for_update()
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-comic-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<ListInfosExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosExcluded<'a>,
    ) -> Result<Vec<ComicInfo>, RegularError> {
        let rows: Vec<ComicRow> = schema::t_comic::table
            .filter(schema::t_comic::f_workset_id.eq(step.spec.workset_id.as_str()))
            .select(ComicRow::as_select())
            .for_update()
            .load(context.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Advance<ReserveCover<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveCover<'a>,
    ) -> Result<ComicCoverReservation, RegularError> {
        let now = OffsetDateTime::now_utc();

        let (prev_key, new_version): (Option<String>, i64) =
            diesel::update(schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.id)))
                .set((
                    schema::t_comic::f_cover_key.eq::<Option<&str>>(None),
                    schema::t_comic::f_cover_uploaded.eq(false),
                    schema::t_comic::f_cover_version.eq(schema::t_comic::f_cover_version + 1),
                    schema::t_comic::f_updated_at.eq(now),
                ))
                .returning((
                    schema::t_comic::f_cover_key,
                    schema::t_comic::f_cover_version,
                ))
                .get_result::<(Option<String>, i64)>(context.conn())
                .await
                .map_err(diesel)?;

        let object_key = crate::complex::comic::ComicComplex::gen_cover_key(
            step.id,
            new_version,
            step.file_extension,
        );

        Ok(ComicCoverReservation {
            object_key,
            prev_object_key: prev_key,
            cover_version: new_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkCoverUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkCoverUploaded<'a>,
    ) -> Result<(), RegularError> {
        let now = OffsetDateTime::now_utc();

        let affected = diesel::update(
            schema::t_comic::table
                .filter(schema::t_comic::f_id.eq(step.id))
                .filter(schema::t_comic::f_cover_version.eq(step.cover_version)),
        )
        .set((
            schema::t_comic::f_cover_uploaded.eq(true),
            schema::t_comic::f_updated_at.eq(now),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        if affected == 0 {
            return Err(expected("error-cover-version-mismatch"));
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Delete<'a>,
    ) -> Result<(), RegularError> {
        diesel::delete(schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.id)))
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<MarkCompleted<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkCompleted<'a>,
    ) -> Result<(), RegularError> {
        let now = OffsetDateTime::now_utc();

        let aspect = ComicAspect::new(now).is_completed(step.is_completed);

        diesel::update(schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.id)))
            .set(&aspect)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<IncrChapterNextIndex<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrChapterNextIndex<'a>,
    ) -> Result<i32, RegularError> {
        let prev: i32 =
            diesel::update(schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.id)))
                .set(
                    schema::t_comic::f_chapter_next_index
                        .eq(schema::t_comic::f_chapter_next_index + 1),
                )
                .returning(schema::t_comic::f_chapter_next_index - 1)
                .get_result(context.conn())
                .await
                .map_err(diesel)?;

        Ok(prev)
    }
}

#[async_trait]
impl<'a> Advance<UpdateChapterCount<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateChapterCount<'a>,
    ) -> Result<(), RegularError> {
        diesel::update(schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.id)))
            .set(schema::t_comic::f_chapter_count.eq(schema::t_comic::f_chapter_count + step.delta))
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &TouchLastActive<'a>,
    ) -> Result<(), RegularError> {
        let now = OffsetDateTime::now_utc();

        let aspect = ComicAspect::new(now).last_active_at(now);

        diesel::update(schema::t_comic::table.filter(schema::t_comic::f_id.eq(step.id)))
            .set(&aspect)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}
