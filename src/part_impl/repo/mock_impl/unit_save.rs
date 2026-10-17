//! Process-local save coordination for use-case tests.

use uuid::Uuid;

use crate::part::repo::unit_save::UnitSaveRepo;
use crate::part_impl::repo::mem_impl::unit_save::UnitSaveCacheGuard;
use crate::part_impl::repo::mock_impl::Mock;

impl UnitSaveRepo for Mock {
    // Share the production guard type for process-local retries.
    type Guard<'a> = UnitSaveCacheGuard<'a>;

    // Reuse the same cache coordination as the production adapter.
    fn lock_save(
        &self,
        save_id: Uuid,
    ) -> impl Future<Output = Self::Guard<'_>> + Send {
        self.unit_saves.lock_save(save_id)
    }
}
