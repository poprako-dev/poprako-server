//! Complex-domain opers for page units.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee,
    check_user_is_chapter_translator_or_proofreader,
    check_user_is_team_member_by_chapter,
};
use crate::model::unit::{
    UnitApplyAck, UnitDiff, UnitIdMapper, UnitIndex, UnitIndexUpdate, UnitOper,
};
use crate::part::repo::step::assignment::GetInfoByChapterIdAndUserId;
use crate::part::repo::step::chapter::GetInfoById as ChapterGetInfoById;
use crate::part::repo::step::comic::GetInfoById as ComicGetInfoById;
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::GetInfoById as WorksetGetInfoById;
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::next_snowflake_id;

/// Domain opers for page units.
pub struct UnitComplex;

impl UnitComplex {
    /// Generates a unique unit identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Validates one compact difference and resolves local create ids.
    pub fn prepare_diff(diff: UnitDiff) -> RegularResult<UnitApplyAck> {
        //
        validate_page_id(&diff.page_id)?;

        let mut local_ids = HashSet::new();

        let mut local_id_map = Vec::new();

        let mut opers = Vec::with_capacity(diff.opers.len());

        for unit_oper in diff.opers {
            match unit_oper {
                //
                UnitOper::Save {
                    local_id,
                    id,
                    payload,
                    before_id,
                } => {
                    //
                    validate_optional_id(&before_id)?;

                    let resolved_id = match (local_id, id) {
                        (Some(local_id), None) => {
                            validate_id(&local_id)?;

                            if !local_ids.insert(local_id.clone()) {
                                return Err(unit_invalid_oper_error());
                            }

                            let unit_id = Self::gen_id();

                            local_id_map.push(UnitIdMapper {
                                local_id: local_id.clone(),
                                unit_id: unit_id.clone(),
                            });

                            unit_id
                        }
                        (None, Some(id)) => {
                            validate_id(&id)?;

                            id
                        }
                        _ => return Err(unit_invalid_oper_error()),
                    };

                    opers.push(UnitOper::Save {
                        local_id: None,
                        id: Some(resolved_id),
                        payload,
                        before_id,
                    });
                }

                UnitOper::Delete { id } => {
                    //
                    validate_id(&id)?;

                    opers.push(UnitOper::Delete { id });
                }
            }
        }

        Ok(UnitApplyAck {
            opers,
            local_id_map,
        })
    }

    /// Applies the ordered opers to the surviving server order in memory and
    /// returns the final id sequence.
    ///
    /// Each save places its unit before `before_id`; `None` or a `before_id`
    /// absent from the surviving order appends the unit to the tail. Delete
    /// removes the unit. Units untouched by the diff keep their relative order.
    pub fn apply_opers_to_order(
        opers: &[UnitOper],
        mut current_order: Vec<String>,
    ) -> Vec<String> {
        //
        for oper in opers {
            match oper {
                //
                UnitOper::Save {
                    id: Some(id),
                    before_id,
                    ..
                } => {
                    //
                    current_order.retain(|surviving_id| surviving_id != id);

                    insert_before(&mut current_order, id, before_id);
                }

                UnitOper::Save { id: None, .. } => {}

                UnitOper::Delete { id } => {
                    current_order.retain(|surviving_id| surviving_id != id);
                }
            }
        }

        current_order
    }

    /// Builds compact index updates from a final id order and the current
    /// persisted indexes.
    pub fn build_index_updates_from_order(
        final_order: &[String],
        current_indexes: &[UnitIndex],
    ) -> Vec<UnitIndexUpdate> {
        //
        let current_map: std::collections::HashMap<&String, i32> =
            current_indexes
                .iter()
                .map(|unit_index| (&unit_index.id, unit_index.index))
                .collect();

        final_order
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                //
                let index = index as i32;

                if current_map.get(id).copied() == Some(index) {
                    return None;
                }

                Some(UnitIndexUpdate {
                    id: id.clone(),
                    index,
                })
            })
            .collect()
    }

    /// Builds compact index updates by compacting the current server order.
    pub fn build_index_updates(
        current_indexes: Vec<UnitIndex>,
    ) -> Vec<UnitIndexUpdate> {
        //
        let mut sorted_indexes = current_indexes;

        sorted_indexes.sort_by(|left, right| {
            left.index
                .cmp(&right.index)
                .then_with(|| left.id.cmp(&right.id))
        });

        compact_index_updates_from_order(sorted_indexes)
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
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            > + for<'a> ProxyExecute<
                GetInfoByChapterIdAndUserId<'a>,
                Error = RegularError,
            >,
    {
        let member_check =
            check_user_is_team_member_by_chapter(proxy, user_id, chapter_id)
                .await;

        if member_check.is_ok() {
            return Ok(());
        }

        match check_user_is_chapter_assignee(proxy, user_id, chapter_id).await {
            Ok(()) => Ok(()),
            Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => Err(unit_list_permission_error()),
            Err(e) => Err(e),
        }
    }

    /// Verify the caller may edit units on a chapter page.
    pub async fn can_user_save_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                GetInfoByChapterIdAndUserId<'a>,
                Error = RegularError,
            >,
    {
        match check_user_is_chapter_translator_or_proofreader(
            proxy, user_id, chapter_id,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => Err(unit_edit_permission_error()),
            Err(e) => Err(e),
        }
    }
}

fn validate_page_id(page_id: &str) -> RegularResult<()> {
    validate_id(page_id)
}

fn validate_id(id: &str) -> RegularResult<()> {
    //
    if id.is_empty() {
        return Err(unit_invalid_oper_error());
    }

    Ok(())
}

fn validate_optional_id(id: &Option<String>) -> RegularResult<()> {
    //
    if id.as_ref().map(|id| id.is_empty()).unwrap_or(false) {
        return Err(unit_invalid_oper_error());
    }

    Ok(())
}

fn insert_before(
    order: &mut Vec<String>,
    id: &str,
    before_id: &Option<String>,
) {
    //
    let Some(before_id) = before_id else {
        //
        order.push(id.to_string());

        return;
    };

    if before_id == id {
        //
        order.push(id.to_string());

        return;
    }

    let Some(position) = order
        .iter()
        .position(|surviving_id| surviving_id == before_id)
    else {
        //
        order.push(id.to_string());

        return;
    };

    order.insert(position, id.to_string());
}

fn compact_index_updates_from_order(
    unit_indexes: Vec<UnitIndex>,
) -> Vec<UnitIndexUpdate> {
    unit_indexes
        .into_iter()
        .enumerate()
        .filter_map(|(index, unit_index)| {
            //
            let index = index as i32;

            if unit_index.index == index {
                return None;
            }

            Some(UnitIndexUpdate {
                id: unit_index.id,
                index,
            })
        })
        .collect()
}

fn unit_invalid_oper_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}

fn unit_list_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-list-permission-required"),
    }
}

fn unit_edit_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-edit-permission-required"),
    }
}

#[cfg(test)]
mod tests;
