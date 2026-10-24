//! Exercise versioned typed persistence against the disposable `PostgreSQL` fixture.

use super::{RdbNucl, RdbRepo, ReptRead, entry};
use crate::model::write::issue_edit::{
    IssueEditPageRepl, IssueEditVariantRename,
};
use crate::part::repo::oper::issue_edit::{
    GetIssuePageRevision, GetReviewVer, ListIssueVariantCounts,
    ListPageIssueInfos, RenameIssueVariants, ReplacePageIssues,
};
use poprako_orchestra::{Nucl as _, OperStep as _};

pub async fn verify(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    chapter_id: &str,
) {
    nucl.coord(async |context| {
        let before = GetReviewVer { chapter_id }.step_on(repo, context).await?;

        let original = GetIssuePageRevision {
            page_artwork_id: chapter_id,
        }
        .step_on(repo, context)
        .await?;

        let replacement = IssueEditPageRepl {
            chapter_id: chapter_id.into(),
            page_artwork_id: chapter_id.into(),
            entries: vec![entry("editor-stable-id", chapter_id, 0)],
        };

        ReplacePageIssues { repl: &replacement }
            .step_on(repo, context)
            .await?;

        let page_revision = GetIssuePageRevision {
            page_artwork_id: chapter_id,
        }
        .step_on(repo, context)
        .await?;

        assert_eq!(page_revision, original + 1);

        let counts = ListIssueVariantCounts { chapter_id }
            .step_on(repo, context)
            .await?;

        assert_eq!(counts.first().unwrap().count, 1);

        let rename = IssueEditVariantRename {
            chapter_id: chapter_id.into(),
            from: "custom category".into(),
            to: "renamed".into(),
        };

        let ids = RenameIssueVariants { rename: &rename }
            .step_on(repo, context)
            .await?;

        assert_eq!(ids, vec![chapter_id]);

        let pages = ListPageIssueInfos {
            page_artwork_id: chapter_id,
        }
        .step_on(repo, context)
        .await?;

        assert_eq!(pages.first().unwrap().id, "editor-stable-id");

        assert_eq!(pages.first().unwrap().variant, "renamed");

        let after = GetReviewVer { chapter_id }.step_on(repo, context).await?;

        assert_eq!(after.gen_no, before.gen_no);

        assert_eq!(after.revision, before.revision + 2);

        let clear = IssueEditPageRepl {
            entries: vec![],
            ..replacement
        };

        ReplacePageIssues { repl: &clear }
            .step_on(repo, context)
            .await?;

        accept!(())
    })
    .await
    .unwrap();
}
