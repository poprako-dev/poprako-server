use super::*;

const USER_ID: &str = "user--00000000001";
const CHAPTER_ID: &str = "chapter--00000000001";
const SOURCE_ID: &str = "page_source--00000000001";
const SOURCE_2_ID: &str = "page_source--00000000002";

// export_visibility(export_translation)(positive): every format preserves visible chain order and contiguous indexes across hidden history.
#[tokio::test]
async fn export_formats_share_visible_units_and_contiguous_indexes() {
    for formats in [
        ExportFormatSpec::LABEL_PLUS,
        ExportFormatSpec::POPRAKO,
        ExportFormatSpec::BOTH,
    ] {
        let mock = Mock::new();

        seed_scope(&mock);

        mock.state.lock().unwrap().objs.clear();

        mock.seed_unit(unit(
            "unit--35727087946",
            SOURCE_ID,
            None,
            " beta ",
            Some(""),
        ));

        let mut hidden_unit_info = unit(
            "unit--79303876260",
            SOURCE_ID,
            Some("unit--35727087946"),
            "hidden translation",
            Some("hidden proofread"),
        );

        hidden_unit_info.hidden_at = Some(OffsetDateTime::now_utc());

        mock.seed_unit(hidden_unit_info);

        mock.seed_unit(unit(
            "unit--34844962250",
            SOURCE_ID,
            Some("unit--79303876260"),
            "alpha",
            Some(" alpha proof "),
        ));

        let exported = export_translation(
            (&mock, &mock, &mock),
            token(USER_ID),
            CHAPTER_ID.into(),
            formats,
            false,
        )
        .await
        .unwrap();

        assert!(exported.raw_idents.is_none());

        assert_eq!(exported.poprako.is_some(), formats.includes_poprako());

        assert_eq!(
            exported.label_plus.is_some(),
            formats.includes_label_plus()
        );

        if let Some(poprako) = exported.poprako {
            assert_visible_poprako(&poprako);
        }

        if let Some(label_plus) = exported.label_plus {
            assert_eq!(
                label_plus,
                concat!(
                    "1,0\n-\n框内\n框外\n-\nExported by PopRaKo Web\n",
                    "\n\n>>>>>>>>[000.jpg]<<<<<<<<\n",
                    "----------------[1]----------------[0.2500,0.7500,1]\n",
                    " alpha proof \n\n",
                    "----------------[2]----------------[0.2500,0.7500,1]\n",
                    " beta \n\n",
                    "\n\n>>>>>>>>[001.jpg]<<<<<<<<\n",
                )
            );
        }
    }
}

fn assert_visible_poprako(
    poprako: &crate::data::view::chapter_port::ChapterTranslationPortView,
) {
    assert_eq!(poprako.pages.len(), 2);

    assert_eq!(
        poprako
            .pages
            .iter()
            .map(|page| page.page_source_id.as_str())
            .collect::<Vec<_>>(),
        [SOURCE_ID, SOURCE_2_ID]
    );

    assert!(poprako.pages.get(1).unwrap().units.is_empty());

    let units = &poprako.pages.first().unwrap().units;

    assert_eq!(units.len(), 2);

    assert_eq!(
        (
            units.first().unwrap().unit_id.as_str(),
            units.first().unwrap().unit_index
        ),
        ("unit--34844962250", 0)
    );

    assert_eq!(
        (
            units.get(1).unwrap().unit_id.as_str(),
            units.get(1).unwrap().unit_index
        ),
        ("unit--35727087946", 1)
    );

    assert_eq!(
        units.first().unwrap().translated_text.as_deref(),
        Some("alpha")
    );

    assert_eq!(
        units.first().unwrap().proofread_text.as_deref(),
        Some(" alpha proof ")
    );

    assert_eq!(
        units.get(1).unwrap().translated_text.as_deref(),
        Some(" beta ")
    );

    assert_eq!(units.get(1).unwrap().proofread_text.as_deref(), Some(""));
}
