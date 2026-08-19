use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::model::page_port::PageTranslationImport;
use crate::model::shared::unit::{UnitRevision, UnitTranslation};
use crate::model::unit_port::UnitTranslationImport;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Internal representation of a LabelPlus unit header.
pub struct LabelPlusUnit {
    /// 0-based index of the unit inside its page.
    index: i32,
    /// X-axis coordinate from the header.
    x_coord: f64,
    /// Y-axis coordinate from the header.
    y_coord: f64,
    /// Whether the unit is a speech bubble.
    is_bubble: bool,
}

/// Normalize a string while preserving all non-empty whitespace and line breaks.
pub fn normalize_string(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

/// Check whether a line is a complete LabelPlus page header.
pub fn is_label_plus_page_header(line: &str) -> bool {
    //
    let line = structural_line(line);

    line.strip_prefix(">>>>>>>>[")
        .and_then(|line| line.strip_suffix("]<<<<<<<<"))
        .is_some_and(|filename| !filename.is_empty())
}

/// Parse a LabelPlus unit header, including its strict index and flag checks.
pub fn parse_label_plus_unit_header(
    line: &str,
) -> BaseRest<Option<LabelPlusUnit>> {
    //
    let line = structural_line(line);

    let Some(rest) = line.strip_prefix("----------------[") else {
        return accept(None);
    };

    let Some((index_text, rest)) = rest.split_once("]----------------[") else {
        return Err(invalid_content("invalid LabelPlus unit separator"));
    };

    let Some(coord_text) = rest.strip_suffix(']') else {
        return Err(invalid_content("missing LabelPlus coordinate suffix"));
    };

    let parts = coord_text.split(',').collect::<Vec<_>>();

    if parts.len() != 3 {
        return Err(invalid_content("invalid LabelPlus coordinate count"));
    }

    let index = index_text
        .parse::<i32>()
        .map_err(|_| invalid_content("invalid LabelPlus unit index"))?;

    if index < 1 {
        return Err(invalid_content("LabelPlus unit index is not positive"));
    }

    let x_coord = parts[0]
        .parse::<f64>()
        .map_err(|_| invalid_content("invalid LabelPlus x coordinate"))?;

    let y_coord = parts[1]
        .parse::<f64>()
        .map_err(|_| invalid_content("invalid LabelPlus y coordinate"))?;

    if !x_coord.is_finite() || !y_coord.is_finite() {
        return Err(invalid_content("LabelPlus coordinate is not finite"));
    }

    let is_bubble = match parts[2] {
        //
        "1" => true,

        "2" => false,

        _ => return Err(invalid_content("invalid LabelPlus bubble flag")),
    };

    accept(Some(LabelPlusUnit {
        index: index - 1,
        x_coord,
        y_coord,
        is_bubble,
    }))
}

/// Flush the buffered LabelPlus unit into the current page.
pub fn flush_label_plus_unit(
    current_page: &mut Option<Vec<UnitTranslationImport>>,
    current_unit: &mut Option<LabelPlusUnit>,
    main_text_lines: &mut Vec<String>,
) -> BaseRest<()> {
    //
    let Some(label_plus_unit) = current_unit.take() else {
        return accept(());
    };

    let Some(page_units) = current_page.as_mut() else {
        return Err(invalid_content("LabelPlus unit has no page"));
    };

    if page_units.len() >= 100 {
        return Err(invalid_content("LabelPlus page has too many units"));
    }

    if page_units
        .iter()
        .any(|unit| unit.index == label_plus_unit.index)
    {
        return Err(invalid_content("duplicate LabelPlus unit index"));
    }

    page_units.push(UnitTranslationImport {
        index: label_plus_unit.index,
        x_coord: label_plus_unit.x_coord,
        y_coord: label_plus_unit.y_coord,
        is_bubble: label_plus_unit.is_bubble,
        main_text: normalize_string(main_text_lines.join("\n")),
        translated_text: None,
        proofread_text: None,
        is_proofread: false,
    });

    main_text_lines.clear();

    accept(())
}

/// Validate and order one parsed LabelPlus page.
pub fn finalize_label_plus_page(
    page: &mut [UnitTranslationImport],
) -> BaseRest<()> {
    //
    if page.len() > 100 {
        return Err(invalid_content("LabelPlus page has too many units"));
    }

    let mut indexes = HashSet::with_capacity(page.len());

    for unit in page.iter() {
        //
        if unit.index < 0 || !indexes.insert(unit.index) {
            return Err(invalid_content("invalid LabelPlus unit index"));
        }
    }

    page.sort_by_key(|unit| unit.index);

    accept(())
}

/// Validate the fixed LabelPlus preamble and its separator layout.
pub fn validate_label_plus_header<'a, I>(lines: &mut I) -> BaseRest<()>
where
    I: Iterator<Item = &'a str>,
{
    //
    let Some(version_line) = lines.next() else {
        return Err(invalid_content("LabelPlus version line is missing"));
    };

    if !version_line
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        return Err(invalid_content("LabelPlus version line is invalid"));
    }

    if lines.next().map(structural_line) != Some("-") {
        return Err(invalid_content("LabelPlus initial separator is missing"));
    }

    let has_content_separator =
        lines.by_ref().map(structural_line).any(|line| line == "-");

    if !has_content_separator {
        return Err(invalid_content("LabelPlus content separator is missing"));
    }

    if lines.next().is_none() {
        return Err(invalid_content("LabelPlus content is missing"));
    }

    accept(())
}

/// Convert the shared PopRaKo document into normalized import pages.
pub fn parse_poprako_document(
    project: ChapterTranslationPortView,
) -> BaseRest<Vec<PageTranslationImport>> {
    //
    if project.pages.len() > 200 {
        return Err(invalid_content("PopRaKo document has too many pages"));
    }

    let page_count = project.pages.len() as i32;

    let mut page_indexes = HashSet::with_capacity(project.pages.len());

    let mut pages = Vec::with_capacity(project.pages.len());

    for page in project.pages {
        //
        if page.page_index < 0
            || page.page_index >= page_count
            || !page_indexes.insert(page.page_index)
        {
            return Err(invalid_content("invalid PopRaKo page index"));
        }

        if page.units.len() > 100 {
            return Err(invalid_content("PopRaKo page has too many units"));
        }

        let mut unit_indexes = HashSet::with_capacity(page.units.len());

        let mut units = Vec::with_capacity(page.units.len());

        for unit in page.units {
            //
            if unit.unit_index < 0 || !unit_indexes.insert(unit.unit_index) {
                return Err(invalid_content("invalid PopRaKo unit index"));
            }

            if !unit.x_coord.is_finite() || !unit.y_coord.is_finite() {
                //
                return Err(invalid_content(
                    "PopRaKo coordinate is not finite",
                ));
            }

            units.push(UnitTranslationImport {
                index: unit.unit_index,
                x_coord: unit.x_coord,
                y_coord: unit.y_coord,
                is_bubble: unit.is_bubble,
                main_text: None,
                translated_text: normalize_option(unit.translated_text),
                proofread_text: normalize_option(unit.proofread_text),
                is_proofread: unit.is_proofread,
            });
        }

        units.sort_by_key(|unit| unit.index);

        pages.push(PageTranslationImport {
            page_index: page.page_index,
            units,
        });
    }

    pages.sort_by_key(|page| page.page_index);

    if pages
        .iter()
        .enumerate()
        .any(|(index, page)| page.page_index != index as i32)
    {
        return Err(invalid_content("PopRaKo page indexes are incomplete"));
    }

    accept(pages)
}

/// Build translated text when the current user has translator permission.
pub fn build_translation(
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

/// Build proofread text when the current user has proofreader permission.
pub fn build_revision(
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

// Remove only spaces and tabs that follow LabelPlus structure lines.
fn structural_line(line: &str) -> &str {
    line.trim_end_matches([' ', '\t'])
}

// Construct the stable client-visible invalid-content error.
fn invalid_content(condition: &str) -> BaseError {
    //
    let err_message = trl("error-invalid-chapter-import-content");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        condition,
        "expected error: chapter import content is invalid",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Normalize an optional text field without changing non-empty content.
fn normalize_option(text: Option<String>) -> Option<String> {
    text.and_then(normalize_string)
}
