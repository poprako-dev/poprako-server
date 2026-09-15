//! Regression coverage for private payloads at use-case tracing boundaries.

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
use crate::usecase::{chapter_port, termbase_port, unit};
use crate::value::termbase::TermbaseScope;
use crate::value::unit::UnitTextPart;

struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
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

// Sensitive inputs must remain absent even when failed operations emit events.
#[tokio::test]
async fn private_payload_spans_record_metadata_without_content() {
    //
    let capture = Capture::default();

    let subscriber = tracing_subscriber::registry().with(capture.clone());

    let secret = "private-payload-sentinel";

    let mock = Mock::new();

    let token = UserToken {
        user_id: "user-1".into(),
    };

    async {
        //
        let instr = ImportChapterTranslationInstr {
            format: ChapterTranslationFormatInstr::LabelPlus,
            mode: ChapterTranslationImportModeInstr::Keep,
            content: secret.into(),
        };

        assert!(
            chapter_port::import_translation::import_translation(
                (&mock, &mock),
                token.clone(),
                instr,
                "chapter-1".into(),
            )
            .await
            .is_err()
        );

        let instr = ImportTermbaseInstr {
            name: secret.into(),
            description: Some(secret.into()),
            terms: vec![ImportTermInstr {
                source: secret.into(),
                targets: vec![secret.into()],
                comment: Some(secret.into()),
            }],
        };

        let scope = TermbaseScope::Team {
            team_id: "team-1".into(),
        };

        assert!(
            termbase_port::import(
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
            phrase: secret.into(),
        };

        assert!(
            unit::search_infos((&mock, &mock), token.clone(), instr)
                .await
                .is_err()
        );

        let instr = TransformChapterUnitsInstr {
            part: UnitTextPart::TranslatedText,
            units: vec![UnitTransformInstr {
                unit_id: "unit-1".into(),
                transforms: vec![UnitTextTransformInstr {
                    origin: secret.into(),
                    target: secret.into(),
                }],
            }],
        };

        assert!(
            unit::transform::transform(
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

    for (name, metadata) in [
        ("import_translation", "content_bytes"),
        ("import", "term_count"),
        ("search_infos", "phrase_bytes"),
        ("transform", "unit_count"),
    ] {
        //
        let fields = spans.get(name).expect("use-case span must be recorded");

        assert!(fields.contains_key("actor_user_id"));

        assert!(fields.contains_key(metadata));

        assert!(!fields.contains_key("instr"));

        assert!(fields.values().all(|value| !value.contains(secret)));
    }
}
