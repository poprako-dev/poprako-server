use std::collections::HashSet;

use crate::model::shared::unit::{UnitRevision, UnitTranslation};
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseResult, accept};
use crate::util::PatchField;
use crate::value::unit::UnitEditPerm;

pub struct UnitComplex;

impl UnitComplex {
    pub fn gen_id() -> String {
        todo!()
    }

    pub fn normalize_edits(
        base_ids: &[&str],
        mut edits: Vec<UnitEdit>,
    ) -> BaseResult<Vec<UnitEdit>> {
        // Move all deletes to the start of edits.
        let _ =
            stable_prior(&mut edits, |e| matches!(e, UnitEdit::Delete { .. }));

        Self::compress_save_edits(&mut edits);

        Self::final_validate_edits(base_ids, &edits)?;

        accept(edits)
    }

    fn compress_save_edits(edits: &mut Vec<UnitEdit>) {
        let mut last = edits.len();

        while last > 0 {
            last -= 1;

            let id = match &edits[last] {
                UnitEdit::Delete { .. } => continue,
                UnitEdit::Save { id, .. } => id.clone(),
            };

            let mut prev = last;

            while prev > 0 {
                prev -= 1;

                match &edits[prev] {
                    UnitEdit::Delete { id: previous_id }
                        if previous_id == &id =>
                    {
                        edits.remove(prev);

                        last -= 1;
                    }

                    UnitEdit::Save { id: prev_id, .. } if prev_id == &id => {
                        {
                            let (head, tail) = edits.split_at_mut(last);

                            let earlier = &mut head[prev];
                            let later = &mut tail[0];

                            Self::merge_save_edits(earlier, later);
                        }

                        edits.remove(prev);
                        last -= 1;
                    }

                    _ => {}
                }
            }
        }

        let mut delete_end = edits
            .iter()
            .take_while(|e| matches!(e, UnitEdit::Delete { .. }))
            .count();

        let mut curr = 0;

        while curr < delete_end {
            let id = match &edits[curr] {
                UnitEdit::Delete { id } => id.clone(),
                UnitEdit::Save { .. } => {
                    unreachable!("delete prefix expected")
                }
            };

            let mut next = curr + 1;

            while next < delete_end {
                let dupl = matches!(
                    &edits[next],
                    UnitEdit::Delete { id: next_id }
                        if next_id == &id
                );

                if dupl {
                    edits.remove(next);
                    delete_end -= 1;
                } else {
                    next += 1;
                }
            }

            curr += 1;
        }
    }

    fn merge_save_edits(earlier: &mut UnitEdit, later: &mut UnitEdit) {
        let (
            UnitEdit::Save {
                id: earlier_id,
                next_id: earlier_next_id,
                translation: earlier_translation,
                revision: earlier_revision,
                ..
            },
            UnitEdit::Save {
                id: later_id,
                next_id: later_next_id,
                translation: later_translation,
                revision: later_revision,
                ..
            },
        ) = (earlier, later)
        else {
            unreachable!("only save edits can be merged");
        };

        debug_assert_eq!(earlier_id, later_id);

        inherit_patch(earlier_next_id, later_next_id);
        inherit_patch(earlier_translation, later_translation);
        inherit_patch(earlier_revision, later_revision);
    }

    fn final_validate_edits(
        base_ids: &[&str],
        edits: &[UnitEdit],
    ) -> BaseResult<()> {
        let base_ids: HashSet<&str> = base_ids.iter().copied().collect();

        let mut delete_ids = HashSet::new();
        let mut save_ids = HashSet::new();

        let mut reached_save = false;

        for edit in edits {
            match edit {
                UnitEdit::Delete { id } => {
                    if reached_save {
                        todo!("delete edit appears after save edit");
                    }

                    if !base_ids.contains(id.as_str()) {
                        todo!("cannot delete an unknown unit");
                    }

                    if !delete_ids.insert(id.as_str()) {
                        todo!("duplicated delete edit");
                    }
                }

                UnitEdit::Save { id, .. } => {
                    reached_save = true;

                    if !save_ids.insert(id.as_str()) {
                        todo!("duplicated save edit after compression");
                    }
                }
            }
        }

        // Delete(id) + Save(id) 应已压缩为 Save(id)。
        if delete_ids.iter().any(|id| save_ids.contains(id)) {
            todo!("delete and save of the same unit were not compressed");
        }

        for edit in edits {
            let UnitEdit::Save {
                id,
                next_id: PatchField::Assign(before_id),
                ..
            } = edit
            else {
                continue;
            };

            let id = id.as_str();
            let before_id = before_id.as_str();

            if before_id == id {
                todo!("unit cannot be placed before itself");
            }

            // Hidden unit 仍是合法序列节点，因此只检查锚点是否存在。
            //
            // base_ids：事务开始前已经存在，包括 visible 和 hidden。
            // save_ids：本批次新建的服务端 ID。
            if !base_ids.contains(before_id) && !save_ids.contains(before_id) {
                todo!("before_id references an unknown unit");
            }
        }

        accept(())
    }

    fn check_save_perm(
        translation: &PatchField<UnitTranslation>,
        revision: &PatchField<UnitRevision>,
        perm: &UnitEditPerm,
    ) -> BaseResult<()> {
        if !translation.is_skip() && !perm.can_translate {
            todo!()
        }
        if !revision.is_skip() && !perm.can_proofread {
            todo!()
        }

        accept(())
    }
}

fn stable_prior<T, P>(slice: &mut [T], mut pred: P) -> usize
where
    P: FnMut(&T) -> bool,
{
    let mut tail = 0;

    for current in 0..slice.len() {
        //
        if pred(&slice[current]) {
            //
            slice[tail..=current].rotate_right(1);

            tail += 1;
        }
    }

    tail
}

fn inherit_patch<T>(earlier: &mut PatchField<T>, later: &mut PatchField<T>) {
    if matches!(later, PatchField::Skip) {
        *later = std::mem::replace(earlier, PatchField::Skip);
    }
}

pub struct UnitPermComplex;

impl UnitPermComplex {
    pub fn ensure_edit_perm(
        perm: &UnitEditPerm,
        edits: &[UnitEdit],
    ) -> BaseResult<()> {
        if !perm.can_translate && !perm.can_proofread {
            todo!()
        }

        for e in edits {
            if let UnitEdit::Save {
                translation,
                revision,
                ..
            } = e
            {
                Self::ensure_save_perm(perm, translation, revision)?;
            }
        }

        todo!()
    }

    fn ensure_save_perm(
        perm: &UnitEditPerm,
        translation: &PatchField<UnitTranslation>,
        revision: &PatchField<UnitRevision>,
    ) -> BaseResult<()> {
        if !translation.is_skip() && !perm.can_translate {
            todo!()
        }
        if !revision.is_skip() && !perm.can_proofread {
            todo!()
        }

        accept(())
    }
}
