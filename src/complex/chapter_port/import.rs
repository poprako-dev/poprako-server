use std::collections::HashMap;

use poprako_util::i18n::trl;

#[cfg(test)]
mod tests;

use crate::model::chapter_port::PoprakoProjectImport;
use crate::model::page_port::{PageTranslationImport, PoprakoPageImport};
use crate::model::unit::{UnitInfo, UnitPayload};
use crate::model::unit_port::UnitTranslationImport;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

/// Chapter import parsing and payload merge rules.
pub struct ChapterImportComplex;

impl ChapterImportComplex {
    /// Parses LabelPlus text into chapter import pages.
    pub fn parse_label_plus(content: &str) -> RootResult<Vec<PageTranslationImport>> {
        let mut lines = content.lines();

        validate_label_plus_header(&mut lines)?;

        let mut pages = Vec::new();
        let mut current_page: Option<Vec<UnitTranslationImport>> = None;
        let mut current_unit: Option<LabelPlusUnit> = None;
        let mut main_text_lines = Vec::new();
        let mut comment_lines = Vec::new();

        for line in lines {
            if is_label_plus_page_header(line) {
                flush_label_plus_unit(
                    &mut current_page,
                    &mut current_unit,
                    &mut main_text_lines,
                    &mut comment_lines,
                )?;

                if let Some(units) = current_page.take() {
                    pages.push(PageTranslationImport { units });
                }

                current_page = Some(Vec::new());

                continue;
            }

            if let Some(unit) = parse_label_plus_unit_header(line)? {
                // FIXME: is_none()
                let Some(_) = current_page else {
                    return Err(args_error("error-invalid-chapter-import-content"));
                };

                flush_label_plus_unit(
                    &mut current_page,
                    &mut current_unit,
                    &mut main_text_lines,
                    &mut comment_lines,
                )?;

                current_unit = Some(unit);

                continue;
            }

            if let Some(comment) = line.strip_prefix("#[翻校注释]：") {
                comment_lines.push(comment.to_string());
                continue;
            }

            if current_unit.is_some() && !line.is_empty() {
                match comment_lines.is_empty() {
                    true => main_text_lines.push(line.to_string()),
                    false => comment_lines.push(line.to_string()),
                }
            }
        }

        flush_label_plus_unit(
            &mut current_page,
            &mut current_unit,
            &mut main_text_lines,
            &mut comment_lines,
        )?;

        if let Some(units) = current_page.take() {
            pages.push(PageTranslationImport { units });
        }

        accept(pages)
    }

    /// Parses PopRaKo JSON text into chapter import pages.
    pub fn parse_poprako(content: &str) -> RootResult<Vec<PageTranslationImport>> {
        let project: PoprakoProjectImport = serde_json::from_str(content)
            .map_err(|_| args_error("error-invalid-chapter-import-content"))?;

        if project.author.trim().is_empty() {
            return Err(args_error("error-invalid-chapter-import-content"));
        }

        if project.title.trim().is_empty() {
            return Err(args_error("error-invalid-chapter-import-content"));
        }

        let pages = project
            .pages
            .into_iter()
            .map(parse_poprako_page)
            .collect::<RootResult<Vec<_>>>()?;

        accept(pages)
    }

    /// Returns an error when imported pages do not match existing pages.
    pub fn validate_page_count(
        imported_page_count: usize,
        existing_page_count: usize,
    ) -> RootResult<()> {
        if imported_page_count != existing_page_count {
            return Err(args_error("error-chapter-import-page-count-mismatch"));
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
    ) -> UnitPayload {
        let mut unit_payload = existing_unit
            .map(payload_from_unit)
            .unwrap_or_else(|| payload_from_import(parsed_unit));

        unit_payload.is_bubble = parsed_unit.is_bubble;
        unit_payload.x_coord = parsed_unit.x_coord;
        unit_payload.y_coord = parsed_unit.y_coord;

        if let Some(translator_comment) = &parsed_unit.translator_comment {
            unit_payload.translator_comment = Some(translator_comment.clone());
        }

        if let Some(proofreader_comment) = &parsed_unit.proofreader_comment {
            unit_payload.proofreader_comment = Some(proofreader_comment.clone());
        }

        match label_plus {
            true => apply_label_plus_text(&mut unit_payload, parsed_unit, user_id, proofreader),
            false => apply_poprako_text(&mut unit_payload, parsed_unit, user_id, proofreader),
        }

        unit_payload
    }
}

struct LabelPlusUnit {
    index: i32,
    x_coord: f64,
    y_coord: f64,
    is_bubble: bool,
}

fn validate_label_plus_header<'a, I>(lines: &mut I) -> RootResult<()>
where
    I: Iterator<Item = &'a str>,
{
    let Some(version_line) = lines.next() else {
        return Err(args_error("error-invalid-chapter-import-content"));
    };

    if !version_line
        .chars()
        .next()
        .map(|value| value.is_ascii_digit())
        .unwrap_or(false)
    {
        return Err(args_error("error-invalid-chapter-import-content"));
    }

    match lines.next() {
        Some("-") => {}
        _ => return Err(args_error("error-invalid-chapter-import-content")),
    }

    let mut found_separator = false;

    for line in lines.by_ref() {
        if line == "-" {
            found_separator = true;

            break;
        }
    }

    if !found_separator {
        return Err(args_error("error-invalid-chapter-import-content"));
    }

    if lines.next().is_none() {
        return Err(args_error("error-invalid-chapter-import-content"));
    }

    accept(())
}

fn is_label_plus_page_header(line: &str) -> bool {
    line.starts_with(">>>>>>>>[") && line.ends_with("]<<<<<<<<")
}

fn parse_label_plus_unit_header(line: &str) -> RootResult<Option<LabelPlusUnit>> {
    let Some(rest) = line.strip_prefix("----------------[") else {
        return accept(None);
    };

    let Some((index_text, rest)) = rest.split_once("]----------------[") else {
        return Err(args_error("error-invalid-chapter-import-content"));
    };

    let Some(coord_text) = rest.strip_suffix(']') else {
        return Err(args_error("error-invalid-chapter-import-content"));
    };

    let parts = coord_text.split(',').collect::<Vec<_>>();

    if parts.len() != 3 {
        return Err(args_error("error-invalid-chapter-import-content"));
    }

    let index = index_text
        .parse::<i32>()
        .map_err(|_| args_error("error-invalid-chapter-import-content"))?;

    let x_coord = parts[0]
        .parse::<f64>()
        .map_err(|_| args_error("error-invalid-chapter-import-content"))?;

    let y_coord = parts[1]
        .parse::<f64>()
        .map_err(|_| args_error("error-invalid-chapter-import-content"))?;

    let is_bubble = match parts[2] {
        "1" => true,
        "2" => false,
        _ => return Err(args_error("error-invalid-chapter-import-content")),
    };

    accept(Some(LabelPlusUnit {
        index: index - 1,
        x_coord,
        y_coord,
        is_bubble,
    }))
}

fn flush_label_plus_unit(
    current_page: &mut Option<Vec<UnitTranslationImport>>,
    current_unit: &mut Option<LabelPlusUnit>,
    main_text_lines: &mut Vec<String>,
    comment_lines: &mut Vec<String>,
) -> RootResult<()> {
    let Some(label_plus_unit) = current_unit.take() else {
        return accept(());
    };

    let Some(page_units) = current_page.as_mut() else {
        return Err(args_error("error-invalid-chapter-import-content"));
    };

    let main_text = normalize_string(main_text_lines.join("\n"));

    let (translator_comment, proofreader_comment) = split_comment(&comment_lines.join("\n"));

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
        translator_comment,
        proofreader_comment,
    });

    main_text_lines.clear();

    comment_lines.clear();

    accept(())
}

fn parse_poprako_page(page: PoprakoPageImport) -> RootResult<PageTranslationImport> {
    if page.image_filename.trim().is_empty() {
        return Err(args_error("error-invalid-chapter-import-content"));
    }

    let mut seen_indexes = HashMap::new();

    let mut units = Vec::with_capacity(page.units.len());

    for unit in page.units {
        if unit.id.trim().is_empty() {
            return Err(args_error("error-invalid-chapter-import-content"));
        }

        if unit.index_in_page < 1 {
            return Err(args_error("error-invalid-chapter-import-content"));
        }

        if !unit.x.is_finite() || !unit.y.is_finite() {
            return Err(args_error("error-invalid-chapter-import-content"));
        }

        if seen_indexes.insert(unit.index_in_page, ()).is_some() {
            return Err(args_error("error-invalid-chapter-import-content"));
        }

        let (translator_comment, proofreader_comment) =
            split_comment(&unit.comment.unwrap_or_default());

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
            translator_comment,
            proofreader_comment,
        });
    }

    units.sort_by(|left, right| left.index.cmp(&right.index));

    accept(PageTranslationImport { units })
}

fn split_comment(comment: &str) -> (Option<String>, Option<String>) {
    if comment.trim().is_empty() {
        return (None, None);
    }

    let mut translator_lines = Vec::new();
    let mut proofreader_lines = Vec::new();
    let mut target = CommentTarget::Translator;

    for line in comment.split('\n') {
        match (
            line.strip_prefix("【翻译】"),
            line.strip_prefix("【校对】"),
            target,
        ) {
            (Some(text), _, _) => {
                translator_lines.push(text.to_string());
                target = CommentTarget::Translator;
            }
            (_, Some(text), _) => {
                proofreader_lines.push(text.to_string());
                target = CommentTarget::Proofreader;
            }
            (None, None, CommentTarget::Translator) => {
                translator_lines.push(line.to_string());
            }
            (None, None, CommentTarget::Proofreader) => {
                proofreader_lines.push(line.to_string());
            }
        }
    }

    (
        normalize_string(translator_lines.join("\n")),
        normalize_string(proofreader_lines.join("\n")),
    )
}

#[derive(Clone, Copy)]
enum CommentTarget {
    Translator,
    Proofreader,
}

fn normalize_option(text: Option<String>) -> Option<String> {
    text.and_then(normalize_string)
}

fn normalize_string(text: String) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

fn payload_from_unit(unit_info: &UnitInfo) -> UnitPayload {
    UnitPayload {
        is_bubble: unit_info.is_bubble,
        is_proofread: unit_info.is_proofread,
        x_coord: unit_info.x_coord,
        y_coord: unit_info.y_coord,
        translated_text: unit_info.translated_text.clone(),
        translator_comment: unit_info.translator_comment.clone(),
        last_translator_id: unit_info.last_translator_id.clone(),
        proofread_text: unit_info.proofread_text.clone(),
        proofreader_comment: unit_info.proofreader_comment.clone(),
        last_proofreader_id: unit_info.last_proofreader_id.clone(),
    }
}

fn payload_from_import(parsed_unit: &UnitTranslationImport) -> UnitPayload {
    UnitPayload {
        is_bubble: parsed_unit.is_bubble,
        is_proofread: parsed_unit.is_proofread,
        x_coord: parsed_unit.x_coord,
        y_coord: parsed_unit.y_coord,
        translated_text: None,
        translator_comment: parsed_unit.translator_comment.clone(),
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: parsed_unit.proofreader_comment.clone(),
        last_proofreader_id: None,
    }
}

fn apply_label_plus_text(
    unit_payload: &mut UnitPayload,
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    proofreader: bool,
) {
    match proofreader {
        true => {
            unit_payload.proofread_text = parsed_unit.main_text.clone();

            if parsed_unit.main_text.is_some() {
                unit_payload.is_proofread = true;
                unit_payload.last_proofreader_id = Some(user_id.into());
            }
        }
        false => {
            unit_payload.translated_text = parsed_unit.main_text.clone();

            if parsed_unit.main_text.is_some() {
                unit_payload.last_translator_id = Some(user_id.into());
            }
        }
    }
}

fn apply_poprako_text(
    unit_payload: &mut UnitPayload,
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    proofreader: bool,
) {
    if let Some(translated_text) = &parsed_unit.translated_text {
        unit_payload.translated_text = Some(translated_text.clone());
        unit_payload.last_translator_id = Some(user_id.into());
    }

    if proofreader {
        if let Some(proofread_text) = &parsed_unit.proofread_text {
            unit_payload.proofread_text = Some(proofread_text.clone());
            unit_payload.is_proofread = true;
            unit_payload.last_proofreader_id = Some(user_id.into());
        }

        if parsed_unit.proofread_text.is_none() && parsed_unit.is_proofread {
            unit_payload.is_proofread = true;
            unit_payload.last_proofreader_id = Some(user_id.into());
        }
    }
}

fn args_error(key: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl(key),
    }
}
