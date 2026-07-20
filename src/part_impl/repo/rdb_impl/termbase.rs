//! Diesel-backed terminology-base repository operations.

use diesel::{
    BoolExpressionMethods as _, ExpressionMethods as _, OptionalExtension as _,
    PgTextExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::termbase::{
    TermbaseEntry, TermbaseInfo, TermbaseInfoListSpec, TermbaseInfoUpdate,
};
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, TouchTermbase,
    UpdateTermbase, UpdateTermbaseTermCount,
};
use crate::part::repo::termbase::TermbaseRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::termbase::{
    TermbaseRow, TermbaseRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_termbase::dsl::*;
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};

#[cfg(all(test, feature = "repo"))]
mod tests;

impl TermbaseRepo<RdbContext> for RdbRepo {}

fn escape_ilike_pattern(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseResult<TermbaseInfo> {
    //
    let row: TermbaseRow = t_termbase
        .filter(f_id.eq(id))
        .select(TermbaseRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-termbase-not-found"))?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<TermbaseInfo> {
    //
    let row: TermbaseRow = t_termbase
        .filter(f_id.eq(id))
        .select(TermbaseRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-termbase-not-found"))?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &TermbaseInfoListSpec,
) -> BaseResult<Vec<TermbaseInfo>> {
    //
    let mut query = t_termbase.select(TermbaseRow::as_select()).into_boxed();

    let (fuzzy_name, offset, limit) = match spec {
        //
        TermbaseInfoListSpec::Team {
            team_id,
            fuzzy_name,
            offset,
            limit,
        } => {
            //
            query = query.filter(f_team_id.eq(team_id));

            (fuzzy_name, offset, limit)
        }

        TermbaseInfoListSpec::Comic {
            team_id,
            comic_id,
            fuzzy_name,
            offset,
            limit,
        } => {
            //
            query =
                query.filter(f_team_id.eq(team_id).or(f_comic_id.eq(comic_id)));

            (fuzzy_name, offset, limit)
        }
    };

    if let Some(fuzzy_name) = fuzzy_name {
        //
        let escaped = escape_ilike_pattern(fuzzy_name);

        let pattern = format!("%{}%", escaped);

        query = query.filter(f_name.ilike(pattern));
    }

    let rows: Vec<TermbaseRow> = query
        .order_by(f_updated_at.desc())
        .offset(*offset as i64)
        .limit(*limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos_excluded(
    conn: &mut RdbConn,
    oper: &ListTermbaseInfosExcluded<'_>,
) -> BaseResult<Vec<TermbaseInfo>> {
    //
    let rows: Vec<TermbaseRow> = match oper {
        //
        ListTermbaseInfosExcluded::Team { team_id } => t_termbase
            .filter(f_team_id.eq(team_id))
            .select(TermbaseRow::as_select())
            .for_update()
            .load(conn)
            .await
            .map_err(diesel)?,

        ListTermbaseInfosExcluded::Comic { comic_id } => t_termbase
            .filter(f_comic_id.eq(comic_id))
            .select(TermbaseRow::as_select())
            .for_update()
            .load(conn)
            .await
            .map_err(diesel)?,
    };

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    termbase_entry: &TermbaseEntry,
) -> BaseResult<TermbaseInfo> {
    //
    let entry = TermbaseRowEntry::from(termbase_entry);

    let row: TermbaseRow = diesel::insert_into(t_termbase)
        .values(&entry)
        .returning(TermbaseRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    update: &TermbaseInfoUpdate,
) -> BaseResult<()> {
    //
    diesel::update(t_termbase.filter(f_id.eq(&update.id)))
        .set((
            f_name.eq(&update.name),
            f_description.eq(update.description.as_deref()),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn update_term_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseResult<()> {
    //
    diesel::update(t_termbase.filter(f_id.eq(id)))
        .set((
            f_term_count.eq(f_term_count + delta),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn touch(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::update(t_termbase.filter(f_id.eq(id)))
        .set(f_updated_at.eq(OffsetDateTime::now_utc()))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_termbase.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl<'a> Run<GetTermbaseInfo<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetTermbaseInfo<'a>,
    ) -> BaseResult<TermbaseInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl<'a> Run<ListTermbaseInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListTermbaseInfos<'a>,
    ) -> BaseResult<Vec<TermbaseInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<'a> Step<CreateTermbase<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateTermbase<'a>,
    ) -> BaseResult<TermbaseInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<GetTermbaseInfo<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTermbaseInfo<'a>,
    ) -> BaseResult<TermbaseInfo> {
        get_info(context.conn(), oper.id).await
    }
}

impl<'a> Step<GetTermbaseInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTermbaseInfoExcluded<'a>,
    ) -> BaseResult<TermbaseInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<'a> Step<ListTermbaseInfosExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListTermbaseInfosExcluded<'a>,
    ) -> BaseResult<Vec<TermbaseInfo>> {
        list_infos_excluded(context.conn(), oper).await
    }
}

impl<'a> Step<UpdateTermbase<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTermbase<'a>,
    ) -> BaseResult<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl<'a> Step<UpdateTermbaseTermCount<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTermbaseTermCount<'a>,
    ) -> BaseResult<()> {
        update_term_count(context.conn(), oper.id, oper.delta).await
    }
}

impl<'a> Step<TouchTermbase<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &TouchTermbase<'a>,
    ) -> BaseResult<()> {
        touch(context.conn(), oper.id).await
    }
}

impl<'a> Step<DeleteTermbase<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTermbase<'a>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
