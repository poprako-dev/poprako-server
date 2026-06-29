//! Complex-domain operations for page units.

use std::collections::HashSet;

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::complex::util::check_user_is_team_member;
use crate::model::role::RoleField;
use crate::model::unit::{
    UnitApplyAck, UnitCounters, UnitIdMapper, UnitInfo, UnitLocalSnapshot, UnitOper,
    UnitServerSnapshot,
};
use crate::part::repo::step::assignment::{AssignmentStep, GetInfoByChapterUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::FindInfoByUserTeamId;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;

/// Domain operations for page units.
pub struct UnitComplex;

impl UnitComplex {
    /// Generates a unique unit identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Applies ordered unit operations to a current page unit sequence.
    pub fn apply_opers(
        page_id: &str,
        current_unit_infos: Vec<UnitInfo>,
        unit_operations: Vec<UnitOper>,
        now: OffsetDateTime,
    ) -> RootResult<UnitApplyAck> {
        let mut unit_infos = current_unit_infos;
        let mut local_id_mappings = Vec::new();
        let mut local_ids = HashSet::new();

        for unit_operation in unit_operations {
            match unit_operation {
                UnitOper::Update { unit } => {
                    validate_server_snapshot(&unit)?;

                    upsert_tail(&mut unit_infos, page_id, unit, now);
                }
                UnitOper::MoveBefore { unit, before_id } => {
                    validate_server_snapshot(&unit)?;

                    validate_before_id(&before_id)?;

                    move_before(&mut unit_infos, page_id, unit, before_id, now);
                }
                UnitOper::InsertBefore { unit, before_id } => {
                    validate_local_snapshot(&unit)?;
                    validate_before_id(&before_id)?;

                    if !local_ids.insert(unit.local_id.clone()) {
                        return Err(unit_invalid_operation_error());
                    }

                    let unit_id = Self::gen_id();

                    insert_before(
                        &mut unit_infos,
                        page_id,
                        unit_id.clone(),
                        unit.clone(),
                        before_id,
                        now,
                    );

                    local_id_mappings.push(UnitIdMapper {
                        local_id: unit.local_id,
                        unit_id,
                    });
                }
                UnitOper::Delete { unit_id } => {
                    validate_id(&unit_id)?;
                    unit_infos.retain(|unit_info| unit_info.id != unit_id);
                }
            }
        }

        refresh_indices(&mut unit_infos);

        let counters = count_units(&unit_infos);

        accept(UnitApplyAck {
            unit_infos,
            id_mapper: local_id_mappings,
            counters,
        })
    }
}

/// Permission-gate operations for page units.
pub struct UnitPermComplex;

impl UnitPermComplex {
    /// Verify the caller may list units on a chapter page.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserTeamId<'a>, Error = RootError>
            + for<'a> ProxyExecute<GetInfoByChapterUserId<'a>, Error = RootError>,
    {
        let chapter_info = proxy
            .execute(&ChapterStep::get_info_by_id(chapter_id))
            .await?;

        let comic_info = proxy
            .execute(&ComicStep::get_info_by_id(&chapter_info.comic_id))
            .await?;

        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
            .await?;

        let member_result = check_user_is_team_member(proxy, user_id, &workset_info.team_id).await;
        if member_result.is_ok() {
            return accept(());
        }

        let assignment_info = proxy
            .execute(&AssignmentStep::get_info_by_chapter_user_id(
                chapter_id, user_id,
            ))
            .await?;

        if assignment_info.is_none() {
            return Err(unit_list_permission_error());
        }

        accept(())
    }

    /// Verify the caller may edit units on a chapter page.
    pub async fn can_user_save_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoByChapterUserId<'a>, Error = RootError>,
    {
        let assignment_info = proxy
            .execute(&AssignmentStep::get_info_by_chapter_user_id(
                chapter_id, user_id,
            ))
            .await?;

        let Some(assignment_info) = assignment_info else {
            return Err(unit_edit_permission_error());
        };

        if !assignment_info
            .roles
            .has_any_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
        {
            return Err(unit_edit_permission_error());
        }

        accept(())
    }
}

fn validate_server_snapshot(unit: &UnitServerSnapshot) -> RootResult<()> {
    validate_id(&unit.id)
}

fn validate_local_snapshot(unit: &UnitLocalSnapshot) -> RootResult<()> {
    validate_id(&unit.local_id)
}

fn validate_id(id: &str) -> RootResult<()> {
    if id.is_empty() {
        return Err(unit_invalid_operation_error());
    }

    accept(())
}

fn validate_before_id(before_id: &Option<String>) -> RootResult<()> {
    if before_id
        .as_ref()
        .map(|value| value.is_empty())
        .unwrap_or(false)
    {
        return Err(unit_invalid_operation_error());
    }

    accept(())
}

fn upsert_tail(
    unit_infos: &mut Vec<UnitInfo>,
    page_id: &str,
    unit: UnitServerSnapshot,
    now: OffsetDateTime,
) {
    match position_by_id(unit_infos, &unit.id) {
        Some(position) => write_existing(&mut unit_infos[position], unit, now),
        None => unit_infos.push(info_from_server(page_id, unit, now)),
    }
}

fn move_before(
    unit_infos: &mut Vec<UnitInfo>,
    page_id: &str,
    unit: UnitServerSnapshot,
    before_id: Option<String>,
    now: OffsetDateTime,
) {
    let mut unit_info = match position_by_id(unit_infos, &unit.id) {
        Some(position) => unit_infos.remove(position),
        None => info_from_server(page_id, unit.clone(), now),
    };
    write_existing(&mut unit_info, unit, now);

    insert_existing_before(unit_infos, unit_info, before_id);
}

fn insert_before(
    unit_infos: &mut Vec<UnitInfo>,
    page_id: &str,
    unit_id: String,
    unit: UnitLocalSnapshot,
    before_id: Option<String>,
    now: OffsetDateTime,
) {
    let unit_info = info_from_local(page_id, unit_id, unit, now);

    insert_existing_before(unit_infos, unit_info, before_id);
}

fn insert_existing_before(
    unit_infos: &mut Vec<UnitInfo>,
    unit_info: UnitInfo,
    before_id: Option<String>,
) {
    let position = before_id
        .filter(|value| value != &unit_info.id)
        .and_then(|value| position_by_id(unit_infos, &value))
        .unwrap_or(unit_infos.len());

    unit_infos.insert(position, unit_info);
}

fn position_by_id(unit_infos: &[UnitInfo], id: &str) -> Option<usize> {
    unit_infos.iter().position(|unit_info| unit_info.id == id)
}

fn write_existing(unit_info: &mut UnitInfo, unit: UnitServerSnapshot, now: OffsetDateTime) {
    unit_info.is_bubble = unit.is_bubble;
    unit_info.is_proofread = unit.is_proofread;
    unit_info.x_coord = unit.x_coord;
    unit_info.y_coord = unit.y_coord;
    unit_info.translated_text = unit.translated_text;
    unit_info.translator_comment = unit.translator_comment;
    unit_info.last_translator_id = unit.last_translator_id;
    unit_info.proofread_text = unit.proofread_text;
    unit_info.proofreader_comment = unit.proofreader_comment;
    unit_info.last_proofreader_id = unit.last_proofreader_id;
    unit_info.updated_at = now;
}

fn info_from_server(page_id: &str, unit: UnitServerSnapshot, now: OffsetDateTime) -> UnitInfo {
    UnitInfo {
        id: unit.id,
        page_id: page_id.into(),
        index: 0,
        is_bubble: unit.is_bubble,
        is_proofread: unit.is_proofread,
        x_coord: unit.x_coord,
        y_coord: unit.y_coord,
        translated_text: unit.translated_text,
        translator_comment: unit.translator_comment,
        last_translator_id: unit.last_translator_id,
        proofread_text: unit.proofread_text,
        proofreader_comment: unit.proofreader_comment,
        last_proofreader_id: unit.last_proofreader_id,
        created_at: now,
        updated_at: now,
    }
}

fn info_from_local(
    page_id: &str,
    unit_id: String,
    unit: UnitLocalSnapshot,
    now: OffsetDateTime,
) -> UnitInfo {
    UnitInfo {
        id: unit_id,
        page_id: page_id.into(),
        index: 0,
        is_bubble: unit.is_bubble,
        is_proofread: unit.is_proofread,
        x_coord: unit.x_coord,
        y_coord: unit.y_coord,
        translated_text: unit.translated_text,
        translator_comment: unit.translator_comment,
        last_translator_id: unit.last_translator_id,
        proofread_text: unit.proofread_text,
        proofreader_comment: unit.proofreader_comment,
        last_proofreader_id: unit.last_proofreader_id,
        created_at: now,
        updated_at: now,
    }
}

fn refresh_indices(unit_infos: &mut [UnitInfo]) {
    for (index, unit_info) in unit_infos.iter_mut().enumerate() {
        unit_info.index = index as i32;
    }
}

fn count_units(unit_infos: &[UnitInfo]) -> UnitCounters {
    UnitCounters {
        total_unit_count: unit_infos.len() as i32,
        translated_unit_count: unit_infos
            .iter()
            .filter(|unit_info| unit_info.is_translated())
            .count() as i32,
        proofread_unit_count: unit_infos
            .iter()
            .filter(|unit_info| unit_info.is_proofread)
            .count() as i32,
    }
}

fn unit_invalid_operation_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-operation"),
    }
}

fn unit_list_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-list-permission-required"),
    }
}

fn unit_edit_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-edit-permission-required"),
    }
}

#[cfg(test)]
mod tests;
