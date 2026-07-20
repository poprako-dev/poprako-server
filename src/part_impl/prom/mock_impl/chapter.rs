use poprako_orchestra::Run as _;

use crate::part::prom::payload::chapter::CheckUploadFinish;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseResult, accept};

/// Process a [`CheckUploadFinish`] task by running the `CompleteChapterRawProvide` repo step.
pub async fn process_check_upload_finish(
    mock: &Mock,
    task: &CheckUploadFinish,
) -> BaseResult<()> {
    //
    mock.run(&CompleteChapterRawProvide {
        id: &task.chapter_id,
    })
    .await?;

    accept(())
}
