//! Complex-domain opers for page units.

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::trl;

use crate::complex::util::check_user_is_team_member;
use crate::model::role::RoleField;
use crate::model::unit::{
    UnitApplyAck, UnitDiff, UnitIdMapper, UnitIndex, UnitIndexUpdate, UnitOper,
};
use crate::part::repo::step::assignment::{AssignmentStep, GetInfoByChapterIdAndUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;

/// Domain opers for page units.
pub struct UnitComplex;

impl UnitComplex {
    /// Generates a unique unit identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Validates one compact difference and resolves local create ids.
    pub fn prepare_diff(diff: UnitDiff) -> RootResult<UnitApplyAck> {
        validate_page_id(&diff.page_id)?;

        let mut candidate_ids = HashSet::new();

        for id in &diff.candidate_order {
            validate_id(id)?;

            if !candidate_ids.insert(id.clone()) {
                return Err(unit_invalid_oper_error());
            }
        }

        let mut required_ids = HashSet::new();
        let mut deleted_ids = HashSet::new();
        let mut local_ids = HashSet::new();
        let mut local_id_map = Vec::new();
        let mut opers = Vec::with_capacity(diff.opers.len());

        for unit_oper in diff.opers {
            match unit_oper {
                UnitOper::Create {
                    local_id,
                    id: _,
                    payload,
                } => {
                    validate_id(&local_id)?;

                    if !local_ids.insert(local_id.clone()) {
                        return Err(unit_invalid_oper_error());
                    }

                    required_ids.insert(local_id.clone());

                    let unit_id = Self::gen_id();

                    local_id_map.push(UnitIdMapper {
                        local_id: local_id.clone(),
                        unit_id: unit_id.clone(),
                    });

                    opers.push(UnitOper::Create {
                        local_id,
                        id: Some(unit_id),
                        payload,
                    });
                }
                UnitOper::Save { id, payload } => {
                    validate_id(&id)?;

                    required_ids.insert(id.clone());

                    opers.push(UnitOper::Save { id, payload });
                }
                UnitOper::Delete { id } => {
                    validate_id(&id)?;

                    deleted_ids.insert(id.clone());

                    opers.push(UnitOper::Delete { id });
                }
            }
        }

        for id in &deleted_ids {
            if required_ids.contains(id) {
                return Err(unit_invalid_oper_error());
            }

            if candidate_ids.contains(id) {
                return Err(unit_invalid_oper_error());
            }
        }

        for id in &required_ids {
            if !candidate_ids.contains(id) {
                return Err(unit_invalid_oper_error());
            }
        }

        let candidate_order = resolve_candidate_order(diff.candidate_order, &local_id_map);

        accept(UnitApplyAck {
            opers,
            local_id_map,
            candidate_order,
        })
    }

    /// Builds compact index updates from resolved candidate order and current indexes.
    pub fn build_index_updates(
        candidate_order: &[String],
        local_id_maps: &[UnitIdMapper],
        current_indexes: Vec<UnitIndex>,
    ) -> Vec<UnitIndexUpdate> {
        let mut sorted_indexes = current_indexes;

        sorted_indexes.sort_by(|left, right| {
            left.index
                .cmp(&right.index)
                .then_with(|| left.id.cmp(&right.id))
        });

        let all_count = sorted_indexes.len();

        if all_count == 0 {
            return Vec::new();
        }

        let local_to_server = local_id_maps
            .iter()
            .map(|unit_id_mapper| {
                (
                    unit_id_mapper.local_id.as_str(),
                    unit_id_mapper.unit_id.as_str(),
                )
            })
            .collect::<HashMap<_, _>>();

        let all_ids = sorted_indexes
            .iter()
            .map(|unit_index| unit_index.id.as_str())
            .collect::<HashSet<_>>();

        let mut anchor_ids = HashSet::new();
        let mut resolved_ids = Vec::new();

        for id in candidate_order {
            let resolved_id = local_to_server
                .get(id.as_str())
                .copied()
                .unwrap_or(id.as_str());

            if !all_ids.contains(resolved_id) {
                continue;
            }

            if !anchor_ids.insert(resolved_id) {
                continue;
            }

            resolved_ids.push(resolved_id);
        }

        let anchor_count = resolved_ids.len();
        let mut slot_extras = HashMap::<usize, Vec<&str>>::new();

        for (rank, unit_index) in sorted_indexes.iter().enumerate() {
            if anchor_ids.contains(unit_index.id.as_str()) {
                continue;
            }

            let mut slot = 0;

            if anchor_count > 0 {
                slot = rank * anchor_count / all_count;

                if slot >= anchor_count {
                    slot = anchor_count - 1;
                }
            }

            slot_extras
                .entry(slot)
                .or_default()
                .push(unit_index.id.as_str());
        }

        let mut final_ids = Vec::with_capacity(all_count);

        for (position, id) in resolved_ids.into_iter().enumerate() {
            final_ids.push(id);

            if let Some(extra_ids) = slot_extras.remove(&position) {
                final_ids.extend(extra_ids);
            }
        }

        if anchor_count == 0 {
            for unit_index in &sorted_indexes {
                final_ids.push(unit_index.id.as_str());
            }
        }

        let old_indexes = sorted_indexes
            .iter()
            .map(|unit_index| (unit_index.id.as_str(), unit_index.index))
            .collect::<HashMap<_, _>>();

        final_ids
            .into_iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let index = index as i32;
                let old_index = old_indexes.get(id)?;

                if *old_index == index {
                    return None;
                }

                Some(UnitIndexUpdate {
                    id: id.into(),
                    index,
                })
            })
            .collect()
    }
}

/// Permission-gate opers for page units.
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
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>
            + for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
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
            .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
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
        P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        let assignment_info = proxy
            .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
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

fn validate_page_id(page_id: &str) -> RootResult<()> {
    validate_id(page_id)
}

fn validate_id(id: &str) -> RootResult<()> {
    if id.is_empty() {
        return Err(unit_invalid_oper_error());
    }

    accept(())
}

fn resolve_candidate_order(
    candidate_order: Vec<String>,
    local_id_maps: &[UnitIdMapper],
) -> Vec<String> {
    let local_to_server = local_id_maps
        .iter()
        .map(|unit_id_map| (unit_id_map.local_id.as_str(), unit_id_map.unit_id.as_str()))
        .collect::<HashMap<_, _>>();

    candidate_order
        .into_iter()
        .map(|id| {
            local_to_server
                .get(id.as_str())
                .copied()
                .unwrap_or(&id)
                .into()
        })
        .collect()
}

fn unit_invalid_oper_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl("error-invalid-unit-oper"),
    }
}

fn unit_list_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-unit-list-permission-required"),
    }
}

fn unit_edit_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-unit-edit-permission-required"),
    }
}

#[cfg(test)]
mod tests;
