use std::collections::HashMap;

use poprako_util::i18n::trl;

use crate::model::chapter_port::ChapterPoprakoProjectImport;
use crate::model::page_port::{PageTranslationImport, PoprakoPageImport};
use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::unit_port::UnitTranslationImport;
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

#[cfg(test)]
// Test cases for chapter-import parsing, translation assembly, and validation.
mod tests;

/// Chapter import parsing and payload merge rules.
pub struct ChapterImportComplex;

impl ChapterImportComplex {
    /// Parses LabelPlus text into chapter import pages.
    pub fn parse_label_plus(
        content: &str,
    ) -> BaseResult<Vec<PageTranslationImport>> {
        //
        let mut lines = content.lines();

        validate_label_plus_header(&mut lines)?;

        let mut pages = Vec::new();

        let mut current_page: Option<Vec<UnitTranslationImport>> = None;

        let mut current_unit: Option<LabelPlusUnit> = None;

        let mut main_text_lines = Vec::new();

        for line in lines {
            //
            if is_label_plus_page_header(line) {
                //
                flush_label_plus_unit(
                    &mut current_page,
                    &mut current_unit,
                    &mut main_text_lines,
                )?;

                if let Some(units) = current_page.take() {
                    pages.push(PageTranslationImport { units });
                }

                current_page = Some(Vec::new());

                continue;
            }

            if let Some(unit) = parse_label_plus_unit_header(line)? {
                //
                if current_page.is_none() {
                    return Err(args_err(
                        "error-invalid-chapter-import-content",
                    ));
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

        if let Some(units) = current_page.take() {
            pages.push(PageTranslationImport { units });
        }

        accept(pages)
    }

    /// Parses PopRaKo JSON text into chapter import pages.
    pub fn parse_poprako(
        content: &str,
    ) -> BaseResult<Vec<PageTranslationImport>> {
        //
        let project: ChapterPoprakoProjectImport =
            serde_json::from_str(content).map_err(|_| {
                args_err("error-invalid-chapter-import-content")
            })?;

        if project.author.trim().is_empty() {
            return Err(args_err("error-invalid-chapter-import-content"));
        }

        if project.title.trim().is_empty() {
            return Err(args_err("error-invalid-chapter-import-content"));
        }

        let pages = project
            .pages
            .into_iter()
            .map(parse_poprako_page)
            .collect::<BaseResult<Vec<_>>>()?;

        accept(pages)
    }

    /// Returns an error when imported pages do not match existing pages.
    pub fn validate_page_count(
        imported_page_count: usize,
        existing_page_count: usize,
    ) -> BaseResult<()> {
        //
        if imported_page_count != existing_page_count {
            return Err(args_err("error-chapter-import-page-count-mismatch"));
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
        let translation =
            build_translation(parsed_unit, user_id, can_translate, label_plus);

        let revision =
            build_revision(parsed_unit, user_id, can_proofread, label_plus);

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

// Internal representation of a parsed LabelPlus unit header containing
// the unit's page-relative index, coordinates, and bubble flag.
struct LabelPlusUnit {
    //
    // 0-based index of unit inside the current page.
    index: i32,

    // X-axis coordinate resolved from the unit header text.
    x_coord: f64,

    // Y-axis coordinate resolved from the unit header text.
    y_coord: f64,

    // Bubble flag from the header (`true` for bubble, `false` for narration).
    is_bubble: bool,
}

// Construct an `Expected::Args` error with the given i18n message key.
fn args_err(key: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(key),
    }
}

// Normalize a string, returning `None` when the trimmed result is empty
// or whitespace-only.
fn normalize_string(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

// Normalize an optional string, returning `None` for empty/whitespace-only
// values.
fn normalize_option(text: Option<String>) -> Option<String> {
    text.and_then(normalize_string)
}

// Check whether a text line matches the LabelPlus page header format
// (`>>>>>>>>[...]<<<<<<<<`).
fn is_label_plus_page_header(line: &str) -> bool {
    line.starts_with(">>>>>>>>[") && line.ends_with("]<<<<<<<<")
}

// Parse a LabelPlus unit header line into its index, coordinates, and
// bubble flag (`1` = bubble, `2` = non-bubble).
fn parse_label_plus_unit_header(
    line: &str,
) -> BaseResult<Option<LabelPlusUnit>> {
    //
    let Some(rest) = line.strip_prefix("----------------[") else {
        return accept(None);
    };

    let Some((index_text, rest)) = rest.split_once("]----------------[") else {
        return Err(args_err("error-invalid-chapter-import-content"));
    };

    let Some(coord_text) = rest.strip_suffix(']') else {
        return Err(args_err("error-invalid-chapter-import-content"));
    };

    let parts = coord_text.split(',').collect::<Vec<_>>();

    if parts.len() != 3 {
        return Err(args_err("error-invalid-chapter-import-content"));
    }

    let index: i32 = index_text
        .parse()
        .map_err(|_| args_err("error-invalid-chapter-import-content"))?;

    let x_coord: f64 = parts[0]
        .parse()
        .map_err(|_| args_err("error-invalid-chapter-import-content"))?;

    let y_coord: f64 = parts[1]
        .parse()
        .map_err(|_| args_err("error-invalid-chapter-import-content"))?;

    let is_bubble = match parts[2] {
        //
        "1" => true,

        "2" => false,

        _ => return Err(args_err("error-invalid-chapter-import-content")),
    };

    accept(Some(LabelPlusUnit {
        index: index - 1,
        x_coord,
        y_coord,
        is_bubble,
    }))
}

// Flush the buffered LabelPlus unit into the current page's unit list,
// building a [`UnitTranslationImport`] from the parsed header and
// accumulated main text lines.
fn flush_label_plus_unit(
    current_page: &mut Option<Vec<UnitTranslationImport>>,
    current_unit: &mut Option<LabelPlusUnit>,
    main_text_lines: &mut Vec<String>,
) -> BaseResult<()> {
    //
    let Some(label_plus_unit) = current_unit.take() else {
        return accept(());
    };

    let Some(page_units) = current_page.as_mut() else {
        return Err(args_err("error-invalid-chapter-import-content"));
    };

    let main_text = normalize_string(main_text_lines.join("\n"));

    page_units.push(UnitTranslationImport {
        index: label_plus_unit.index,
        x_coord: label_plus_unit.x_coord,
        y_coord: label_plus_unit.y_coord,
        is_bubble: label_plus_unit.is_bubble,
        main_text,
        translated_text: None,
        proofread_text: None,
        is_proofread: false,
    });

    main_text_lines.clear();

    accept(())
}

// Parse a single PopRaKo JSON page import into a [`PageTranslationImport`],
// validating required fields, unique indexes, and finite coordinates.
fn parse_poprako_page(
    page: PoprakoPageImport,
) -> BaseResult<PageTranslationImport> {
    //
    if page.image_filename.trim().is_empty() {
        return Err(args_err("error-invalid-chapter-import-content"));
    }

    let mut seen_indexes = HashMap::new();

    let mut units = Vec::with_capacity(page.units.len());

    for unit in page.units {
        //
        if unit.id.trim().is_empty() {
            return Err(args_err("error-invalid-chapter-import-content"));
        }

        if unit.index_in_page < 1 {
            return Err(args_err("error-invalid-chapter-import-content"));
        }

        if !unit.x.is_finite() || !unit.y.is_finite() {
            return Err(args_err("error-invalid-chapter-import-content"));
        }

        if seen_indexes.insert(unit.index_in_page, ()).is_some() {
            return Err(args_err("error-invalid-chapter-import-content"));
        }

        units.push(UnitTranslationImport {
            index: unit.index_in_page - 1,
            x_coord: unit.x,
            y_coord: unit.y,
            is_bubble: unit.is_inbox,
            main_text: None,
            translated_text: normalize_option(unit.translated_text),
            proofread_text: normalize_option(unit.prooved_text),
            is_proofread: unit.is_prooved,
        });
    }

    units.sort_by_key(|left| left.index);

    accept(PageTranslationImport { units })
}

// Validate the LabelPlus header: version line starting with a digit,
// a `-` separator line, content lines ending with a `-` separator,
// and at least one trailing line.
fn validate_label_plus_header<'a, I>(lines: &mut I) -> BaseResult<()>
where
    I: Iterator<Item = &'a str>,
{
    let Some(version_line) = lines.next() else {
        return Err(args_err("error-invalid-chapter-import-content"));
    };

    if !version_line
        .chars()
        .next()
        .map(|value| value.is_ascii_digit())
        .unwrap_or(false)
    {
        return Err(args_err("error-invalid-chapter-import-content"));
    }

    if lines.next() != Some("-") {
        return Err(args_err("error-invalid-chapter-import-content"));
    }

    let mut found_separator = false;

    for line in lines.by_ref() {
        if line == "-" {
            //
            found_separator = true;

            break;
        }
    }

    if !found_separator {
        return Err(args_err("error-invalid-chapter-import-content"));
    }

    if lines.next().is_none() {
        return Err(args_err("error-invalid-chapter-import-content"));
    }

    accept(())
}

// Build translated text for a parsed unit when translation is allowed.
// For LabelPlus imports, source text is reused as translation text.
fn build_translation(
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    can_translate: bool,
    label_plus: bool,
) -> Option<UnitTranslation> {
    //
    if !can_translate {
        return None;
    }

    let translated_text = match label_plus {
        //
        true => parsed_unit.main_text.clone(),

        false => parsed_unit.translated_text.clone(),
    };

    translated_text.map(|translated_text| UnitTranslation {
        translated_text,
        last_translator_id: user_id.to_string(),
    })
}

// Build revision payload for a parsed unit when proofread is allowed.
// For LabelPlus imports, source text is also used as proofread text.
fn build_revision(
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    can_proofread: bool,
    label_plus: bool,
) -> Option<UnitRevision> {
    //
    if !can_proofread {
        return None;
    }

    let proofread_text = match label_plus {
        //
        true => parsed_unit.main_text.clone(),

        false => parsed_unit.proofread_text.clone(),
    };

    let is_proofread =
        label_plus && proofread_text.is_some() || parsed_unit.is_proofread;

    if proofread_text.is_none() && !is_proofread {
        return None;
    }

    Some(UnitRevision {
        is_proofread,
        proofread_text,
        last_proofreader_id: user_id.to_string(),
    })
}
