//! RDB-backed member repository — [`Execute`] and [`Advance`] implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::member::{MemberForm, MemberListSpec, MemberRoleUpdate};
use crate::part::repo::step::member::{
    Create, Delete, FindInfoByUserIdAndTeamId, GetInfoById, GetInfoExcluded, ListInfos,
    ListInfosByUserIdExcluded, TouchLastActive, UpdateRole, UpdateUserNickname,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::member::{MemberAspect, MemberEntry, MemberRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::RootError;
use crate::value::role::{RoleField, RoleMask};

fn role_timestamps_from_mask(
    roles: RoleMask,
    now: OffsetDateTime,
) -> (
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
) {
    let ts = |field: RoleField| -> Option<OffsetDateTime> {
        match roles.has_any_role(&[field]) {
            true => Some(now),
            false => None,
        }
    };

    (
        ts(RoleField::RAW_PROVIDER),
        ts(RoleField::TRANSLATOR),
        ts(RoleField::PROOFREADER),
        ts(RoleField::TYPESETTER),
        ts(RoleField::REDRAWER),
        ts(RoleField::REVIEWER),
        ts(RoleField::PUBLISHER),
        ts(RoleField::ADMIN),
        None,
    )
}

fn member_entry_from_form<'a>(form: &'a MemberForm, now: OffsetDateTime) -> MemberEntry<'a> {
    let (
        raw_provider,
        translator,
        proofreader,
        typesetter,
        redrawer,
        reviewer,
        publisher,
        admin,
        assistant,
    ) = role_timestamps_from_mask(form.roles, now);

    MemberEntry {
        f_id: &form.id,
        f_user_id: &form.user_id,
        f_user_nickname: &form.user_nickname,
        f_team_id: &form.team_id,
        f_assigned_raw_provider_at: raw_provider,
        f_assigned_translator_at: translator,
        f_assigned_proofreader_at: proofreader,
        f_assigned_typesetter_at: typesetter,
        f_assigned_redrawer_at: redrawer,
        f_assigned_reviewer_at: reviewer,
        f_assigned_publisher_at: publisher,
        f_assigned_admin_at: admin,
        f_assigned_assistant_at: assistant,
        f_user_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    }
}

// ── Non-transactional ──────────────────────────────────────────────────────

#[async_trait]
impl<'a> Execute<FindInfoByUserIdAndTeamId<'a>> for RdbRepo {
    type Error = RootError;

    async fn execute(
        &self,
        step: &FindInfoByUserIdAndTeamId<'a>,
    ) -> Result<
        <FindInfoByUserIdAndTeamId<'_> as poprako_transactional::step::Step>::Output,
        Self::Error,
    > {
        let mut conn = self.conn().await?;

        let row = schema::t_member::table
            .filter(schema::t_member::f_user_id.eq(step.user_id))
            .filter(schema::t_member::f_team_id.eq(step.team_id))
            .select(MemberRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?;

        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RootError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<<ListInfos<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let rows = match step.spec {
            MemberListSpec::Team {
                team_id,
                fuzzy_nickname,
                role,
                offset,
                limit,
                ..
            } => {
                let mut query = schema::t_member::table
                    .filter(schema::t_member::f_team_id.eq(team_id.as_str()))
                    .select(MemberRow::as_select())
                    .into_boxed();

                match fuzzy_nickname {
                    Some(nick) => {
                        query = query
                            .filter(schema::t_member::f_user_nickname.ilike(format!("%{}%", nick)));
                    }
                    None => {}
                }

                match role {
                    Some(_rf) => {}
                    None => {}
                }

                query
                    .order_by(schema::t_member::f_user_last_active_at.desc())
                    .offset((*offset) as i64)
                    .limit((*limit) as i64)
                    .load(conn.conn())
                    .await
                    .map_err(diesel)?
            }
            MemberListSpec::User {
                owner_id,
                offset,
                limit,
                ..
            } => schema::t_member::table
                .filter(schema::t_member::f_user_id.eq(owner_id.as_str()))
                .select(MemberRow::as_select())
                .order_by(schema::t_member::f_user_last_active_at.desc())
                .offset((*offset) as i64)
                .limit((*limit) as i64)
                .load(conn.conn())
                .await
                .map_err(diesel)?,
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RootError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<<GetInfoById<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let row = schema::t_member::table
            .filter(schema::t_member::f_id.eq(step.id))
            .select(MemberRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-member-not-found"))?;

        Ok(row.into())
    }
}

// ── Transactional ──────────────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> Result<<Create<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let now = OffsetDateTime::now_utc();
        let entry = member_entry_from_form(step.form, now);

        let row = diesel::insert_into(schema::t_member::table)
            .values(&entry)
            .returning(MemberRow::as_returning())
            .get_result(context.conn())
            .await
            .map_err(diesel)?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<UpdateUserNickname<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateUserNickname<'a>,
    ) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();

        let aspect = MemberAspect::new(now).user_nickname(step.user_nickname);

        diesel::update(
            schema::t_member::table.filter(schema::t_member::f_user_id.eq(step.user_id)),
        )
        .set(&aspect)
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &TouchLastActive<'a>,
    ) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();

        let aspect = MemberAspect::new(now).user_last_active_at(now);

        diesel::update(
            schema::t_member::table.filter(schema::t_member::f_user_id.eq(step.user_id)),
        )
        .set(&aspect)
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByUserIdExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosByUserIdExcluded<'a>,
    ) -> Result<
        <ListInfosByUserIdExcluded<'a> as poprako_transactional::step::Step>::Output,
        Self::Error,
    > {
        let rows: Vec<MemberRow> = schema::t_member::table
            .filter(schema::t_member::f_user_id.eq(step.user_id))
            .select(MemberRow::as_select())
            .for_update()
            .load(context.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Advance<FindInfoByUserIdAndTeamId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FindInfoByUserIdAndTeamId<'a>,
    ) -> Result<
        <FindInfoByUserIdAndTeamId<'a> as poprako_transactional::step::Step>::Output,
        Self::Error,
    > {
        let row = schema::t_member::table
            .filter(schema::t_member::f_user_id.eq(step.user_id))
            .filter(schema::t_member::f_team_id.eq(step.team_id))
            .select(MemberRow::as_select())
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?;

        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<<GetInfoExcluded<'a> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let row = schema::t_member::table
            .filter(schema::t_member::f_id.eq(step.id))
            .select(MemberRow::as_select())
            .for_update()
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-member-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<UpdateRole<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateRole<'a>,
    ) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();
        let update: &MemberRoleUpdate = step.member_role_update;

        let (
            raw_provider,
            translator,
            proofreader,
            typesetter,
            redrawer,
            reviewer,
            publisher,
            admin,
            assistant,
        ) = role_timestamps_from_mask(update.roles, now);

        let mut aspect = MemberAspect::new(now);
        aspect = aspect
            .assigned_raw_provider_at(raw_provider)
            .assigned_translator_at(translator)
            .assigned_proofreader_at(proofreader)
            .assigned_typesetter_at(typesetter)
            .assigned_redrawer_at(redrawer)
            .assigned_reviewer_at(reviewer)
            .assigned_publisher_at(publisher)
            .assigned_admin_at(admin)
            .assigned_assistant_at(assistant);

        diesel::update(
            schema::t_member::table.filter(schema::t_member::f_id.eq(update.id.as_str())),
        )
        .set(&aspect)
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(&self, context: &mut RdbContext, step: &Delete<'a>) -> Result<(), RootError> {
        diesel::delete(schema::t_member::table.filter(schema::t_member::f_id.eq(step.id)))
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}
