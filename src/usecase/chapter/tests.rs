// Shared fixture setup helpers for chapter test suites.
mod fixture;
// Team administration without chapter assignments.
mod administration;
// Preset-assignment scenarios and perm transitions.
mod preset_assignment;
// Workflow-stage transition assertions for chapter usecases.
mod extra;
mod stage;

// list_infos(list_infos)(positive): team member can list chapters sorted by newest index with pagination.
// list_infos(list_infos)(negative): non-member cannot list chapters.
// get_info(get_info)(positive): team member can read a chapter.
// get_info(get_info)(negative): missing chapter returns an argument error.
// get_pinned(get_pinned)(positive): pinned chapter is returned and missing pinned chapter returns none.
// get_pinned(get_pinned)(negative): non-member cannot read pinned chapter.
// create(create)(positive): team admin creates pinned chapter, unpins previous chapter, updates comic, and only creates explicit worker assignments.
// create(create)(positive): creator presets contain worker roles only.
// create(create)(negative): non-admin creation rolls back.
// create(create)(negative): creator cannot preset a role missing from team membership.
// update_info(update_info)(positive): team admin can update metadata without an assignment.
// update_info(update_info)(negative): non-admin cannot update metadata.
// mark_pinned(mark_pinned)(positive): team admin pins the chapter and unpins its sibling.
// mark_pinned(mark_pinned)(negative): non-admin cannot pin a chapter.
// update_stage(update_stage)(positive): team admin can advance any stage without a role holder.
// update_stage(update_stage)(negative): reviewer cannot advance another role's stage.
// update_stage(update_stage)(negative): invalid workflow transition is rejected.
// update_stage(update_stage)(positive): publishing enqueues page image deletion.
// update_stage(update_stage)(positive): role holder advances own stage.
// update_stage(update_stage)(positive): admin can advance when no role holder exists.
// update_stage(update_stage)(positive): admin with workflow role advances when they hold the role.
// update_stage(update_stage)(positive): admin reverts stage even when no role holder exists.
// delete(delete)(positive): admin deletes chapter descendants, enqueues page image deletion, repins latest remaining chapter, and decrements comic.
// delete(delete)(negative): non-admin delete rolls back.

use super::*;

use fixture::*;

use crate::data::instr::chapter::{
    CreateChapterInstr, UpdateChapterInfoInstr, UpdateChapterStageInstr,
};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::usecase::chapter::stage::update_stage;
use crate::value::chapter::stage::Stage;
use crate::value::role::{RoleField, RoleMask};
