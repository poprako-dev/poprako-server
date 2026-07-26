//! Complex-domain opers for page units.

use std::collections::{HashMap, HashSet};

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee,
    check_user_is_chapter_translator_or_proofreader,
    check_user_is_team_member_by_chapter,
};
use crate::model::unit::{
    UnitApplyAck, UnitBody, UnitDiff, UnitIdMapper, UnitIndex, UnitIndexUpdate,
    UnitMdf,
};
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

#[cfg(test)]
mod tests;

/// Domain opers for page units.
pub struct UnitComplex;

impl UnitComplex {
    /// Generates a unique unit identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Validates one compact difference and resolves local create ids.
    pub fn prepare_diff(diff: UnitDiff) -> BaseResult<UnitApplyAck> {
        //
        validate_page_id(&diff.page_id)?;

        let mut local_ids = HashSet::new();

        let mut local_id_map = Vec::new();

        let mut opers = Vec::with_capacity(diff.mdfs.len());

        for unit_oper in diff.mdfs {
            match unit_oper {
                //
                UnitMdf::Create {
                    id,
                    body: payload,
                    before_id,
                } => {
                    //
                    validate_optional_id(&before_id)?;

                    validate_id(&id)?;

                    validate_payload(&payload)?;

                    if !local_ids.insert(id.clone()) {
                        return Err(unit_invalid_oper_err());
                    }

                    let unit_id = Self::gen_id();

                    local_id_map.push(UnitIdMapper {
                        local_id: id,
                        unit_id: unit_id.clone(),
                    });

                    opers.push(UnitMdf::Create {
                        id: unit_id,
                        body: payload,
                        before_id,
                    });
                }

                UnitMdf::Save {
                    id,
                    body: payload,
                    before_id,
                } => {
                    //
                    validate_id(&id)?;

                    validate_optional_id(&before_id)?;

                    validate_payload(&payload)?;

                    opers.push(UnitMdf::Save {
                        id,
                        body: payload,
                        before_id,
                    });
                }

                UnitMdf::Delete { id } => {
                    //
                    validate_id(&id)?;

                    opers.push(UnitMdf::Delete { id });
                }
            }
        }

        accept(UnitApplyAck {
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
        opers: &[UnitMdf],
        mut current_order: Vec<String>,
    ) -> Vec<String> {
        //
        for oper in opers {
            match oper {
                //
                UnitMdf::Create { id, before_id, .. }
                | UnitMdf::Save { before_id, id, .. } => {
                    //
                    current_order.retain(|surviving_id| surviving_id != id);

                    insert_before(&mut current_order, id, before_id);
                }

                UnitMdf::Delete { id } => {
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
        let current_map: HashMap<&String, i32> = current_indexes
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
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        let member_check =
            check_user_is_team_member_by_chapter(proxy, user_id, chapter_id)
                .await;

        if member_check.is_ok() {
            return accept(());
        }

        match check_user_is_chapter_assignee(proxy, user_id, chapter_id).await {
            //
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => Err(unit_list_permission_err()),

            Err(e) => Err(e),
        }
    }

    /// Verify the caller may edit units on a chapter page.
    pub async fn ensure_user_can_save_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        match check_user_is_chapter_translator_or_proofreader(
            proxy, user_id, chapter_id,
        )
        .await
        {
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => Err(unit_edit_permission_err()),

            Err(e) => Err(e),
        }
    }
}

/// Validate a page ID string (delegates to [`validate_id`]).
fn validate_page_id(page_id: &str) -> BaseResult<()> {
    validate_id(page_id)
}

/// Validate a non-empty identifier, returning an args error for empty strings.
fn validate_id(id: &str) -> BaseResult<()> {
    //
    if id.is_empty() {
        return Err(unit_invalid_oper_err());
    }

    accept(())
}

/// Validate an optional identifier — rejects `Some("")` but allows `None`.
fn validate_optional_id(id: &Option<String>) -> BaseResult<()> {
    //
    if id.as_ref().map(|id| id.is_empty()).unwrap_or(false) {
        return Err(unit_invalid_oper_err());
    }

    accept(())
}

/// Validate editor identifiers required by non-empty unit text fields.
fn validate_payload(payload: &UnitBody) -> BaseResult<()> {
    //
    validate_text_editor(
        &payload.translated_text,
        &payload.last_translator_id,
    )?;

    validate_text_editor(
        &payload.proofread_text,
        &payload.last_proofreader_id,
    )?;

    accept(())
}

/// Require a non-empty editor identifier when text is non-empty.
fn validate_text_editor(
    text: &Option<String>,
    editor_id: &Option<String>,
) -> BaseResult<()> {
    //
    let has_text = text.as_ref().map(|text| !text.is_empty()).unwrap_or(false);

    match (has_text, editor_id.as_deref()) {
        //
        (true, Some(editor_id)) => validate_id(editor_id),

        (true, None) => Err(unit_invalid_oper_err()),

        (false, _) => accept(()),
    }
}

/// Insert `id` before `before_id` in the order vector. Appends to the end
/// when `before_id` is `None`, equals `id`, or is not found.
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

/// Build index updates by enumerating sorted unit indexes and emitting
/// updates only for positions that differ from the stored index.
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

/// Construct an "invalid unit operation" args error.
fn unit_invalid_oper_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}

/// Construct a "unit list permission required" error.
fn unit_list_permission_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-list-permission-required"),
    }
}

/// Construct a "unit edit permission required" error.
fn unit_edit_permission_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-edit-permission-required"),
    }
}
