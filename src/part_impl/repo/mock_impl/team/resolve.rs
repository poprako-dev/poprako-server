//! In-memory team ownership projections.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::part::repo::oper::team::ResolveTeamId;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, unrecoverable,
};
use crate::result::{BaseError, BaseRest, accept};

fn resolve_comic_team_id(state: &MockState, id: &str) -> BaseRest<String> {
    //
    let comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == id)
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let workset_info = state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == comic_info.workset_id)
        .ok_or_else(|| {
            unrecoverable(
                "[resolve_comic_team_id] comic references missing workset",
            )
        })?;

    accept(workset_info.team_id.clone())
}

fn resolve_chapter_team_id(state: &MockState, id: &str) -> BaseRest<String> {
    //
    let chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == id)
        .ok_or_else(|| expected("error-chapter-not-found"))?;

    let comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == chapter_info.comic_id)
        .ok_or_else(|| {
            unrecoverable(
                "[resolve_chapter_team_id] chapter references missing comic",
            )
        })?;

    let workset_info = state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == comic_info.workset_id)
        .ok_or_else(|| {
            unrecoverable(
                "[resolve_chapter_team_id] comic references missing workset",
            )
        })?;

    accept(workset_info.team_id.clone())
}

fn resolve_team_id(
    state: &MockState,
    oper: &ResolveTeamId<'_>,
) -> BaseRest<String> {
    match oper {
        //
        ResolveTeamId::Comic { id } => resolve_comic_team_id(state, id),

        ResolveTeamId::Chapter { id } => resolve_chapter_team_id(state, id),
    }
}

impl Run<ResolveTeamId<'_>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ResolveTeamId<'_>,
    ) -> Result<String, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        resolve_team_id(&state, oper)
    }
}

impl Step<ResolveTeamId<'_>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ResolveTeamId<'_>,
    ) -> Result<String, Self::Error> {
        resolve_team_id(&context.state, oper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use poprako_orchestra::Nucl as _;
    use time::OffsetDateTime;

    use poprako_util::i18n::trl;

    use crate::model::read::proj::chapter::ChapterInfo;
    use crate::model::read::proj::comic::ComicInfo;
    use crate::model::read::proj::workset::WorksetInfo;
    use crate::result::ExpectedVariant;
    use crate::value::chapter::StageMask;
    use crate::value::image::{ImageExt, ImageHash};

    fn seed_scope(mock: &Mock) {
        //
        let time = OffsetDateTime::now_utc();

        mock.seed_workset(WorksetInfo {
            id: "workset-1".into(),
            team_id: "team-1".into(),
            index: 0,
            name: "workset".into(),
            description: None,
            comic_count: 1,
            created_at: time,
            updated_at: time,
        });

        mock.seed_comic(ComicInfo {
            id: "comic-1".into(),
            workset_id: "workset-1".into(),
            index: 0,
            title: "comic".into(),
            author: "author".into(),
            description: None,
            cover_key: None,
            is_cover_uploaded: false,
            cover_version: 0,
            cover_hash: ImageHash::default(),
            cover_ext: ImageExt::Png,
            chapter_count: 1,
            creator_id: "user-1".into(),
            workset: None,
            team: None,
            creator: None,
            last_active_at: time,
            created_at: time,
            updated_at: time,
        });

        mock.seed_chapter(ChapterInfo {
            id: "chapter-1".into(),
            comic_id: "comic-1".into(),
            comic: None,
            is_pinned: false,
            index: 0,
            subtitle: "chapter".into(),
            page_count: 0,
            total_unit_count: 0,
            translated_unit_count: 0,
            proofread_unit_count: 0,
            stages: StageMask::try_from(0u32).ok().unwrap(),
            creator_id: "user-1".into(),
            creator: None,
            created_at: time,
            updated_at: time,
        });
    }

    fn assert_expected(error: BaseError, message_key: &str) {
        let BaseError::Expected { variant, message } = error else {
            panic!("expected client-visible resource error");
        };

        assert!(matches!(variant, ExpectedVariant::Args));

        assert_eq!(message, trl(message_key));
    }

    #[tokio::test]
    async fn run_resolves_comic_and_reports_missing_root() {
        //
        let mock = Mock::new();

        seed_scope(&mock);

        let team_id = mock
            .run(&ResolveTeamId::Comic { id: "comic-1" })
            .await
            .ok()
            .unwrap();

        assert_eq!(team_id, "team-1");

        let team_id = mock
            .run(&ResolveTeamId::Chapter { id: "chapter-1" })
            .await
            .ok()
            .unwrap();

        assert_eq!(team_id, "team-1");

        let error = mock
            .run(&ResolveTeamId::Comic {
                id: "missing-comic",
            })
            .await
            .err()
            .unwrap();

        assert_expected(error, "error-comic-not-found");

        let error = mock
            .run(&ResolveTeamId::Chapter {
                id: "missing-chapter",
            })
            .await
            .err()
            .unwrap();

        assert_expected(error, "error-chapter-not-found");
    }

    #[tokio::test]
    async fn step_resolves_chapter_and_reports_missing_root() {
        //
        let mock = Mock::new();

        seed_scope(&mock);

        let repo = mock.clone();

        mock.coord(async move |context| {
            //
            let team_id = repo
                .step(context, &ResolveTeamId::Chapter { id: "chapter-1" })
                .await?;

            assert_eq!(team_id, "team-1");

            let team_id = repo
                .step(context, &ResolveTeamId::Comic { id: "comic-1" })
                .await?;

            assert_eq!(team_id, "team-1");

            let error = repo
                .step(
                    context,
                    &ResolveTeamId::Chapter {
                        id: "missing-chapter",
                    },
                )
                .await
                .err()
                .unwrap();

            assert_expected(error, "error-chapter-not-found");

            let error = repo
                .step(
                    context,
                    &ResolveTeamId::Comic {
                        id: "missing-comic",
                    },
                )
                .await
                .err()
                .unwrap();

            assert_expected(error, "error-comic-not-found");

            Ok::<(), BaseError>(())
        })
        .await
        .ok()
        .unwrap();
    }
}
