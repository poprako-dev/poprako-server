use std::collections::HashMap;

use poprako_util::i18n::trl;

use crate::model::chapter_port::ChapterPoprakoProjectImport;
use crate::model::page_port::{PageTranslationImport, PoprakoPageImport};
use crate::model::unit::{UnitBody, UnitInfo};
use crate::model::unit_port::UnitTranslationImport;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

#[cfg(test)]
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

    /// Builds an import payload by merging parsed text with an existing unit.
    pub fn build_unit_payload(
        parsed_unit: &UnitTranslationImport,
        existing_unit: Option<&UnitInfo>,
        user_id: &str,
        proofreader: bool,
        label_plus: bool,
    ) -> UnitBody {
        //
        let mut unit_payload = existing_unit
            .map(payload_from_unit)
            .unwrap_or_else(|| payload_from_import(parsed_unit));

        unit_payload.is_bubble = parsed_unit.is_bubble;

        unit_payload.x_coord = parsed_unit.x_coord;

        unit_payload.y_coord = parsed_unit.y_coord;

        match label_plus {
            //
            true => apply_label_plus_text(
                &mut unit_payload,
                parsed_unit,
                user_id,
                proofreader,
            ),

            false => apply_poprako_text(
                &mut unit_payload,
                parsed_unit,
                user_id,
                proofreader,
            ),
        }

        unit_payload
    }
}

/// Internal representation of a parsed LabelPlus unit header containing
/// the unit's page-relative index, coordinates, and bubble flag.
struct LabelPlusUnit {
    //
    index: i32,
    x_coord: f64,
    y_coord: f64,
    is_bubble: bool,
}

/// Validate the LabelPlus header: version line starting with a digit,
/// a `-` separator line, content lines ending with a `-` separator,
/// and at least one trailing line.
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

/// Check whether a text line matches the LabelPlus page header format
/// (`>>>>>>>>[...]<<<<<<<<`).
fn is_label_plus_page_header(line: &str) -> bool {
    line.starts_with(">>>>>>>>[") && line.ends_with("]<<<<<<<<")
}

/// Parse a LabelPlus unit header line into its index, coordinates, and
/// bubble flag (`1` = bubble, `2` = non-bubble).
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

/// Flush the buffered LabelPlus unit into the current page's unit list,
/// building a [`UnitTranslationImport`] from the parsed header and
/// accumulated main text lines.
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
        id: None,
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

/// Parse a single PopRaKo JSON page import into a [`PageTranslationImport`],
/// validating required fields, unique indexes, and finite coordinates.
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
            id: Some(unit.id),
            index: unit.index_in_page,
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

/// Normalize an optional string, returning `None` for empty/whitespace-only
/// values.
fn normalize_option(text: Option<String>) -> Option<String> {
    text.and_then(normalize_string)
}

/// Normalize a string, returning `None` when the trimmed result is empty
/// or whitespace-only.
fn normalize_string(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

/// Build a [`UnitPayload`] from an existing persisted [`UnitInfo`],
/// preserving all stored text and metadata fields.
fn payload_from_unit(unit_info: &UnitInfo) -> UnitBody {
    UnitBody {
        is_bubble: unit_info.is_bubble,
        is_proofread: unit_info.is_proofread,
        x_coord: unit_info.x_coord,
        y_coord: unit_info.y_coord,
        translated_text: unit_info.translated_text.clone(),
        last_translator_id: unit_info.last_translator_id.clone(),
        proofread_text: unit_info.proofread_text.clone(),
        last_proofreader_id: unit_info.last_proofreader_id.clone(),
    }
}

/// Build a fresh [`UnitPayload`] from imported unit data with no existing
/// text or metadata.
fn payload_from_import(parsed_unit: &UnitTranslationImport) -> UnitBody {
    UnitBody {
        is_bubble: parsed_unit.is_bubble,
        is_proofread: parsed_unit.is_proofread,
        x_coord: parsed_unit.x_coord,
        y_coord: parsed_unit.y_coord,
        translated_text: None,
        last_translator_id: None,
        proofread_text: None,
        last_proofreader_id: None,
    }
}

/// Apply LabelPlus main text to the unit payload, assigning it as
/// proofread or translated text based on the caller's role.
fn apply_label_plus_text(
    unit_payload: &mut UnitBody,
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    proofreader: bool,
) {
    match proofreader {
        //
        true => {
            //
            unit_payload.proofread_text = parsed_unit.main_text.clone();

            if parsed_unit.main_text.is_some() {
                //
                unit_payload.is_proofread = true;

                unit_payload.last_proofreader_id = Some(user_id.into());
            }
        }

        false => {
            //
            unit_payload.translated_text = parsed_unit.main_text.clone();

            if parsed_unit.main_text.is_some() {
                unit_payload.last_translator_id = Some(user_id.into());
            }
        }
    }
}

/// Apply PopRaKo JSON text fields to the unit payload, writing translated
/// and proofread text according to the caller's role.
fn apply_poprako_text(
    unit_payload: &mut UnitBody,
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    proofreader: bool,
) {
    //
    if let Some(translated_text) = &parsed_unit.translated_text {
        //
        unit_payload.translated_text = Some(translated_text.clone());

        unit_payload.last_translator_id = Some(user_id.into());
    }

    if proofreader {
        //
        if let Some(proofread_text) = &parsed_unit.proofread_text {
            //
            unit_payload.proofread_text = Some(proofread_text.clone());

            unit_payload.is_proofread = true;

            unit_payload.last_proofreader_id = Some(user_id.into());
        }

        if parsed_unit.proofread_text.is_none() && parsed_unit.is_proofread {
            //
            unit_payload.is_proofread = true;

            unit_payload.last_proofreader_id = Some(user_id.into());
        }
    }
}

/// Construct an `Expected::Args` error with the given i18n message key.
fn args_err(key: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(key),
    }
}
