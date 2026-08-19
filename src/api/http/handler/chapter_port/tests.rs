use super::*;

use axum::extract::Query;
use axum::http::Uri;

use crate::data::instr::chapter_port::ChapterTranslationFormatInstr;

// translation_export_query_accepts_snake_case_format(TranslationExportQuery)(positive): export query formats use the public snake_case enum contract.
#[test]
fn translation_export_query_accepts_snake_case_format() {
    let uri = "http://localhost/translations/export?format=label_plus"
        .parse::<Uri>()
        .unwrap();

    let query = Query::<TranslationExportQuery>::try_from_uri(&uri)
        .unwrap()
        .0;

    assert!(matches!(
        query.format,
        ChapterTranslationFormatInstr::LabelPlus
    ));
}

// translation_export_query_rejects_kebab_case_format(TranslationExportQuery)(negative): the export query no longer accepts the storage-oriented kebab-case value.
#[test]
fn translation_export_query_rejects_kebab_case_format() {
    let uri = "http://localhost/translations/export?format=label-plus"
        .parse::<Uri>()
        .unwrap();

    assert!(Query::<TranslationExportQuery>::try_from_uri(&uri).is_err());
}
