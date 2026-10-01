use super::*;

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

        mock.seed_unit(unit("unit-b", "page-1", None, " beta ", Some("")));

        let mut hidden_unit_info = unit(
            "unit-hidden",
            "page-1",
            Some("unit-b"),
            "hidden translation",
            Some("hidden proofread"),
        );

        hidden_unit_info.hidden_at = Some(OffsetDateTime::now_utc());

        mock.seed_unit(hidden_unit_info);

        mock.seed_unit(unit(
            "unit-a",
            "page-1",
            Some("unit-hidden"),
            "alpha",
            Some(" alpha proof "),
        ));

        let exported = export_translation(
            (&mock, &mock, &mock),
            token("user-1"),
            "chapter-1".into(),
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
            assert_eq!(poprako.pages.len(), 2);

            assert_eq!(poprako.pages[0].page_id, "page-1");

            assert_eq!(poprako.pages[1].page_id, "page-2");

            assert!(poprako.pages[1].units.is_empty());

            let units = &poprako.pages[0].units;

            assert_eq!(units.len(), 2);

            assert_eq!(
                (units[0].unit_id.as_str(), units[0].unit_index),
                ("unit-a", 0)
            );

            assert_eq!(
                (units[1].unit_id.as_str(), units[1].unit_index),
                ("unit-b", 1)
            );

            assert_eq!(units[0].translated_text.as_deref(), Some("alpha"));

            assert_eq!(
                units[0].proofread_text.as_deref(),
                Some(" alpha proof ")
            );

            assert_eq!(units[1].translated_text.as_deref(), Some(" beta "));

            assert_eq!(units[1].proofread_text.as_deref(), Some(""));
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
