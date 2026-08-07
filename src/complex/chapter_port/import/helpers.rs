use std::collections::HashMap;

use poprako_util::i18n::trl;

use crate::model::page_port::{PageTranslationImport, PoprakoPageImport};
use crate::model::shared::unit::{UnitRevision, UnitTranslation};
use crate::model::unit_port::UnitTranslationImport;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Internal representation of a parsed LabelPlus unit header containing
/// the unit's page-relative index, coordinates, and bubble flag.
pub struct LabelPlusUnit {
    /// 0-based index of unit inside the current page.
    index: i32,

    /// X-axis coordinate resolved from the unit header text.
    x_coord: f64,

    /// Y-axis coordinate resolved from the unit header text.
    y_coord: f64,

    /// Bubble flag from the header (`true` for bubble, `false` for narration).
    is_bubble: bool,
}

/// Normalize a string, returning `None` when the trimmed result is empty
/// or whitespace-only.
pub fn normalize_string(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

/// Check whether a text line matches the LabelPlus page header format
/// (`>>>>>>>>[...]<<<<<<<<`).
pub fn is_label_plus_page_header(line: &str) -> bool {
    line.starts_with(">>>>>>>>[") && line.ends_with("]<<<<<<<<")
}

/// Parse a LabelPlus unit header line into its index, coordinates, and
/// bubble flag (`1` = bubble, `2` = non-bubble).
pub fn parse_label_plus_unit_header(
    line: &str,
) -> BaseRest<Option<LabelPlusUnit>> {
    //
    let Some(rest) = line.strip_prefix("----------------[") else {
        return accept(None);
    };

    let Some((index_text, rest)) = rest.split_once("]----------------[") else {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            line = %line,
            "expected error: chapter import unit header separator is invalid",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    };

    let Some(coord_text) = rest.strip_suffix(']') else {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            line = %line,
            "expected error: chapter import coordinate suffix is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    };

    let parts = coord_text.split(',').collect::<Vec<_>>();

    if parts.len() != 3 {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            line = %line,
            part_count = parts.len(),
            "expected error: chapter import coordinate count is invalid",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let index = index_text.parse::<i32>().map_err(|error| {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            line = %line,
            raw_index = %index_text,
            parse_err = ?error,
            "expected error: chapter import unit index is invalid",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    let x_coord = parts[0].parse::<f64>().map_err(|error| {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            line = %line,
            raw_x_coord = %parts[0],
            parse_err = ?error,
            "expected error: chapter import x coordinate is invalid",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    let y_coord = parts[1].parse::<f64>().map_err(|error| {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            line = %line,
            raw_y_coord = %parts[1],
            parse_err = ?error,
            "expected error: chapter import y coordinate is invalid",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    let is_bubble = match parts[2] {
        //
        "1" => true,

        "2" => false,

        _ => {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                line = %line,
                raw_bubble_flag = %parts[2],
                "expected error: chapter import bubble flag is invalid",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }
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
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            unit_index = label_plus_unit.index,
            main_text_line_count = main_text_lines.len(),
            "expected error: chapter import unit has no current page",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
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

/// Parse a single PopRaKo JSON page import into a [`PageTranslationImport`],
/// validating required fields, unique indexes, and finite coordinates.
pub fn parse_poprako_page(
    page: PoprakoPageImport,
) -> BaseRest<PageTranslationImport> {
    //
    if page.image_filename.trim().is_empty() {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            image_filename = %page.image_filename,
            unit_count = page.units.len(),
            "expected error: chapter import image filename is empty",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let (mut seen_indexes, mut units) =
        (HashMap::new(), Vec::with_capacity(page.units.len()));

    for unit in page.units {
        //
        if unit.id.trim().is_empty() {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                unit_id = %unit.id,
                unit_index = unit.index_in_page,
                "expected error: chapter import unit id is empty",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if unit.index_in_page < 1 {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                unit_id = %unit.id,
                unit_index = unit.index_in_page,
                "expected error: chapter import unit index is invalid",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if !unit.x.is_finite() || !unit.y.is_finite() {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                unit_id = %unit.id,
                unit_index = unit.index_in_page,
                x_coord = unit.x,
                y_coord = unit.y,
                "expected error: chapter import unit coordinate is not finite",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if seen_indexes.insert(unit.index_in_page, ()).is_some() {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                unit_id = %unit.id,
                unit_index = unit.index_in_page,
                "expected error: duplicate chapter import unit index",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
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

/// Validate the LabelPlus header: version line starting with a digit,
/// a `-` separator line, content lines ending with a `-` separator,
/// and at least one trailing line.
pub fn validate_label_plus_header<'a, I>(lines: &mut I) -> BaseRest<()>
where
    I: Iterator<Item = &'a str>,
{
    let Some(version_line) = lines.next() else {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            operation = "validate_label_plus_header",
            condition = "missing_version_line",
            "expected error: chapter import version line is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    };

    if !version_line
        .chars()
        .next()
        .map(|value| value.is_ascii_digit())
        .unwrap_or(false)
    {
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            version_line = %version_line,
            "expected error: chapter import version line is invalid",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if lines.next() != Some("-") {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            version_line = %version_line,
            condition = "missing_initial_separator",
            "expected error: chapter import initial separator is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let mut found_separator = false;

    for line in lines.by_ref() {
        //
        if line == "-" {
            //
            found_separator = true;

            break;
        }
    }

    if !found_separator {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            version_line = %version_line,
            condition = "missing_content_separator",
            "expected error: chapter import content separator is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if lines.next().is_none() {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            version_line = %version_line,
            condition = "missing_content",
            "expected error: chapter import content is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(())
}

/// Build translated text for a parsed unit when translation is allowed.
/// For LabelPlus imports, source text is reused as translation text.
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

/// Build revision payload for a parsed unit when proofread is allowed.
/// For LabelPlus imports, source text is also used as proofread text.
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

// Normalize an optional string, returning `None` for empty/whitespace-only
// values.
fn normalize_option(text: Option<String>) -> Option<String> {
    text.and_then(normalize_string)
}
