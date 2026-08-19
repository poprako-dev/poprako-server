// Chapter import parsing helpers.
mod helpers;

#[cfg(test)]
// Test cases for chapter-import parsing, translation assembly, and validation.
mod tests;

use poprako_util::i18n::trl;

use crate::complex::chapter_port::import::helpers::{
    LabelPlusUnit, build_revision, build_translation, finalize_label_plus_page,
    flush_label_plus_unit, is_label_plus_page_header,
    parse_label_plus_unit_header, parse_poprako_document,
    validate_label_plus_header,
};
use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::model::page_port::PageTranslationImport;
use crate::model::shared::unit::UnitCoord;
use crate::model::unit_port::UnitTranslationImport;
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Chapter import parsing and payload merge rules.
pub struct ChapterImportComplex;

impl ChapterImportComplex {
    /// Parses LabelPlus text into chapter import pages.
    pub fn parse_label_plus(
        content: &str,
    ) -> BaseRest<Vec<PageTranslationImport>> {
        //
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);

        let mut lines = content
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line));

        validate_label_plus_header(&mut lines)?;

        let (
            mut pages,
            mut current_page,
            mut current_unit,
            mut main_text_lines,
        ) = (
            Vec::new(),
            None::<Vec<UnitTranslationImport>>,
            None::<LabelPlusUnit>,
            Vec::new(),
        );

        for line in lines {
            //
            if is_label_plus_page_header(line) {
                //
                flush_label_plus_unit(
                    &mut current_page,
                    &mut current_unit,
                    &mut main_text_lines,
                )?;

                if let Some(mut units) = current_page.take() {
                    //
                    finalize_label_plus_page(&mut units)?;

                    pages.push(PageTranslationImport {
                        page_index: pages.len() as i32,
                        units,
                    });
                }

                current_page = Some(Vec::new());

                continue;
            }

            let structural_line = line.trim_end_matches([' ', '\t']);

            if structural_line.starts_with(">>>>>>>>") {
                //
                let err_message = trl("error-invalid-chapter-import-content");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    "expected error: LabelPlus structure line is malformed",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            if let Some(unit) = parse_label_plus_unit_header(line)? {
                //
                if current_page.is_none() {
                    //
                    let err_message =
                        trl("error-invalid-chapter-import-content");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        line = %line,
                        "expected error: chapter import unit appears before page",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    });
                }

                flush_label_plus_unit(
                    &mut current_page,
                    &mut current_unit,
                    &mut main_text_lines,
                )?;

                current_unit = Some(unit);

                continue;
            }

            if current_unit.is_some() && !line.is_empty() {
                main_text_lines.push(line.to_string());
            }
        }

        flush_label_plus_unit(
            &mut current_page,
            &mut current_unit,
            &mut main_text_lines,
        )?;

        if let Some(mut units) = current_page.take() {
            //
            finalize_label_plus_page(&mut units)?;

            pages.push(PageTranslationImport {
                page_index: pages.len() as i32,
                units,
            });
        }

        if pages.len() > 200 {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                page_count = pages.len(),
                "expected error: LabelPlus document has too many pages",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(pages)
    }

    /// Parses PopRaKo JSON text into chapter import pages.
    pub fn parse_poprako(
        content: &str,
    ) -> BaseRest<Vec<PageTranslationImport>> {
        //
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);

        let project =
            serde_json::from_str::<ChapterTranslationPortView>(content)
                .map_err(|error| {
                    //
                    let err_message =
                        trl("error-invalid-chapter-import-content");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        input_length = content.len(),
                        parse_err = ?error,
                        operation = "parse_poprako",
                        "expected error: chapter import JSON is invalid",
                    );

                    BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    }
                })?;

        parse_poprako_document(project)
    }

    /// Returns an error when imported pages do not match existing pages.
    pub fn validate_page_count(
        imported_page_count: usize,
        existing_page_count: usize,
    ) -> BaseRest<()> {
        //
        if imported_page_count != existing_page_count {
            //
            let err_message = trl("error-chapter-import-page-count-mismatch");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                imported_page_count = imported_page_count,
                existing_page_count = existing_page_count,
                "expected error: chapter import page count mismatch",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(())
    }

    /// Builds one Unit Create from parsed import content.
    pub fn build_unit_create(
        parsed_unit: &UnitTranslationImport,
        unit_id: String,
        user_id: &str,
        can_translate: bool,
        can_proofread: bool,
        label_plus: bool,
    ) -> UnitEdit {
        //
        let (translation, revision) = (
            build_translation(parsed_unit, user_id, can_translate, label_plus),
            build_revision(parsed_unit, user_id, can_proofread, label_plus),
        );

        UnitEdit::Create {
            id: unit_id,
            next_id: None,
            is_bubble: parsed_unit.is_bubble,
            coord: UnitCoord {
                x_coord: parsed_unit.x_coord,
                y_coord: parsed_unit.y_coord,
            },
            translation,
            revision,
        }
    }
}
