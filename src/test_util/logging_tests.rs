#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

//! Regression coverage for ordinary instruction payloads in use-case spans.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing::instrument::WithSubscriber as _;
use tracing::span::{Attributes, Id};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt as _};

use crate::data::instr::chapter_port::{
    ChapterTranslationFormatInstr, ChapterTranslationImportModeInstr,
    ImportChapterTranslationInstr,
};
use crate::data::instr::termbase_port::{ImportTermInstr, ImportTermbaseInstr};
use crate::data::instr::unit::{
    SearchChapterUnitInfosInstr, TransformChapterUnitsInstr,
    UnitTextTransformInstr, UnitTransformInstr,
};
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::usecase::chapter_port::import_translation as chapter_port_import_translation_usecase;
use crate::usecase::unit::transform as unit_transform_usecase;
use crate::usecase::{
    termbase_port as termbase_port_usecase, unit as unit_usecase,
};
use crate::value::termbase::TermbaseScope;
use crate::value::unit::UnitTextPart;

struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    #[expect(
        clippy::uninlined_format_args,
        reason = "Repository formatting keeps interpolation arguments explicit"
    )]
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0.insert(field.name().into(), format!("{:?}", value));
    }
}

#[derive(Clone, Default)]
struct Capture {
    spans: Arc<Mutex<BTreeMap<String, BTreeMap<String, String>>>>,
}

impl<S> Layer<S> for Capture
where
    S: Subscriber,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, _: &Id, _: Context<'_, S>) {
        //
        let mut fields = Fields(BTreeMap::new());

        attrs.record(&mut fields);

        self.spans
            .lock()
            .unwrap()
            .insert(attrs.metadata().name().into(), fields.0);
    }
}

// Use cases record the complete instruction through its declared Debug policy.
#[tokio::test]
async fn ordinary_instruction_spans_keep_business_content() {
    //
    let capture = Capture::default();

    let subscriber = tracing_subscriber::registry().with(capture.clone());

    let content = "business-content-sentinel";

    let mock = Mock::new();

    let token = UserToken {
        user_id: "user-1".into(),
    };

    async {
        //
        let instr = ImportChapterTranslationInstr {
            format: ChapterTranslationFormatInstr::LabelPlus,
            mode: ChapterTranslationImportModeInstr::Keep,
            content: content.into(),
        };

        assert!(
            chapter_port_import_translation_usecase::import_translation(
                (&mock, &mock),
                token.clone(),
                instr,
                "chapter-1".into(),
            )
            .await
            .is_err()
        );

        let instr = ImportTermbaseInstr {
            name: content.into(),
            description: Some(content.into()),
            terms: vec![ImportTermInstr {
                source: content.into(),
                targets: vec![content.into()],
                comment: Some(content.into()),
            }],
        };

        let scope = TermbaseScope::Team {
            team_id: "team-1".into(),
        };

        assert!(
            termbase_port_usecase::import(
                (&mock, &mock),
                token.clone(),
                scope,
                false,
                instr,
            )
            .await
            .is_err()
        );

        let instr = SearchChapterUnitInfosInstr {
            chapter_id: "chapter-1".into(),
            part: UnitTextPart::TranslatedText,
            phrase: content.into(),
        };

        assert!(
            unit_usecase::search_infos((&mock, &mock), token.clone(), instr)
                .await
                .is_err()
        );

        let instr = TransformChapterUnitsInstr {
            part: UnitTextPart::TranslatedText,
            units: vec![UnitTransformInstr {
                unit_id: "unit-1".into(),
                transforms: vec![UnitTextTransformInstr {
                    origin: content.into(),
                    target: content.into(),
                }],
            }],
        };

        assert!(
            unit_transform_usecase::transform(
                (&mock, &mock),
                token,
                "chapter-1".into(),
                instr,
            )
            .await
            .is_err()
        );
    }
    .with_subscriber(subscriber)
    .await;

    let spans = capture.spans.lock().unwrap();

    for name in ["import_translation", "import", "search_infos", "transform"] {
        //
        let fields = spans.get(name).expect("use-case span must be recorded");

        assert!(fields.contains_key("actor_user_id"));

        let instr = fields.get("instr").expect("instruction must be recorded");

        assert!(instr.contains(content));

        assert!(!instr.contains("[REDACTED]"));
    }

    drop(spans);
}
