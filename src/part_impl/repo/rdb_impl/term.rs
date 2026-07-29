//! Diesel-backed terminology-entry repository operations.

use diesel::ExpressionMethods as _;
use diesel::OptionalExtension as _;
use diesel::PgTextExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::SelectableHelper as _;
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::term::{
    TermEntry, TermInfo, TermInfoListSpec, TermInfoUpdate,
};
use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, DeleteTerms, GetTermInfo, GetTermInfoExcluded,
    ListTermInfos, UpdateTerm,
};
use crate::part::repo::term::TermRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::term::{TermRow, TermRowEntry};
use crate::part_impl::repo::rdb_impl::schema::t_term::dsl::*;
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};

#[cfg(all(test, feature = "repo"))]
mod tests;

impl TermRepo<RdbContext> for RdbRepo {}

fn escape_ilike_pattern(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseResult<TermInfo> {
    let row: TermRow = t_term
        .filter(f_id.eq(id))
        .select(TermRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-term-not-found"))?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<TermInfo> {
    let row: TermRow = t_term
        .filter(f_id.eq(id))
        .select(TermRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-term-not-found"))?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &TermInfoListSpec,
) -> BaseResult<Vec<TermInfo>> {
    let mut query = t_term
        .filter(f_termbase_id.eq(&spec.termbase_id))
        .select(TermRow::as_select())
        .into_boxed();

    if let Some(fuzzy_source) = &spec.fuzzy_source {
        let escaped = escape_ilike_pattern(fuzzy_source);

        let pattern = format!("%{}%", escaped);

        query = query.filter(f_source.ilike(pattern));
    }

    let rows: Vec<TermRow> = query
        .order_by(f_updated_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    term_entry: &TermEntry,
) -> BaseResult<TermInfo> {
    let entry = TermRowEntry::from(term_entry);

    let row: TermRow = diesel::insert_into(t_term)
        .values(&entry)
        .returning(TermRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    update: &TermInfoUpdate,
) -> BaseResult<()> {
    let targets = update
        .targets
        .iter()
        .map(|target| Some(target.as_str()))
        .collect::<Vec<_>>();

    diesel::update(t_term.filter(f_id.eq(&update.id)))
        .set((
            f_source.eq(&update.source),
            f_targets.eq(targets),
            f_comment.eq(update.comment.as_deref()),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    diesel::delete(t_term.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn delete_terms(conn: &mut RdbConn, termbase_id: &str) -> BaseResult<()> {
    diesel::delete(t_term.filter(f_termbase_id.eq(termbase_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl<'a> Run<GetTermInfo<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetTermInfo<'a>) -> BaseResult<TermInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl<'a> Run<ListTermInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListTermInfos<'a>) -> BaseResult<Vec<TermInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<'a> Step<CreateTerm<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateTerm<'a>,
    ) -> BaseResult<TermInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<GetTermInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTermInfoExcluded<'a>,
    ) -> BaseResult<TermInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<'a> Step<UpdateTerm<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTerm<'a>,
    ) -> BaseResult<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl<'a> Step<DeleteTerm<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTerm<'a>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<'a> Step<DeleteTerms<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTerms<'a>,
    ) -> BaseResult<()> {
        delete_terms(context.conn(), oper.termbase_id).await
    }
}
