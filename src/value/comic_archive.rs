//! Calendar-month values used by comic archive export and retention.

use std::collections::HashSet;

use serde::Serialize;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

use poprako_util::i18n::trl;

use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

#[cfg(test)]
mod tests;

/// Maximum number of month slots accepted by one export request.
pub const MAX_EXPORT_MONTHS: usize = 12;

/// Complete immutable comic payload serialized once when archiving.
#[derive(Serialize)]
pub struct ArchivedComicPayload {
    pub source_comic_id: String,
    pub workset: ArchivedWorksetPayload,
    pub index: i32,
    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub chapter_count: i32,
    pub chapter_next_index: i32,
    pub creator_id: String,
    pub last_active_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub chapters: Vec<ArchivedChapterPayload>,
}

/// Immutable workset payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedWorksetPayload {
    pub id: String,
    pub team_id: String,
    pub index: i32,
    pub name: String,
    pub description: Option<String>,
    pub comic_count: i32,
    pub comic_next_index: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Immutable chapter payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedChapterPayload {
    pub source_chapter_id: String,
    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,
    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
    pub stages: u32,
    pub creator_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub assignments: Vec<ArchivedAssignmentPayload>,
    pub pages: Vec<ArchivedPagePayload>,
}

/// Immutable assignment payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedAssignmentPayload {
    pub source_assignment_id: String,
    pub user_id: String,
    pub roles: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub user: ArchivedUserPayload,
}

/// Immutable user payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedUserPayload {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: u32,
    pub is_sadmin: bool,
    pub last_active_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Immutable page payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedPagePayload {
    pub source_page_id: String,
    pub index: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub units: Vec<ArchivedUnitPayload>,
}

/// Immutable unit payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedUnitPayload {
    pub source_unit_id: String,
    pub index: i32,
    pub is_bubble: bool,
    pub is_proofread: bool,
    pub x_coord: f64,
    pub y_coord: f64,
    pub translated_text: Option<String>,
    pub last_translator_id: Option<String>,
    pub proofread_text: Option<String>,
    pub last_proofreader_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// One validated UTC calendar-month range.
pub struct ComicArchiveMonth {
    pub label: String,

    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl ComicArchiveMonth {
    /// Parses distinct retained month labels and resolves their UTC bounds.
    pub fn parse_retained(
        labels: Vec<String>,
        now: OffsetDateTime,
    ) -> BaseResult<Vec<Self>> {
        //
        if labels.is_empty() || labels.len() > MAX_EXPORT_MONTHS {
            return Err(args("error-invalid-comic-archive-month-count"));
        }

        let current = (now.year(), u8::from(now.month()));

        let earliest = (now.year() - 1, u8::from(now.month()));

        let mut unique_labels = HashSet::with_capacity(labels.len());

        let mut months = Vec::with_capacity(labels.len());

        for label in labels {
            //
            if !unique_labels.insert(label.clone()) {
                return Err(args("error-duplicate-comic-archive-month"));
            }

            let (year, month) = parse_label(&label)?;

            if (year, month) < earliest || (year, month) > current {
                return Err(args("error-comic-archive-month-not-retained"));
            }

            months.push(Self::new(label, year, month)?);
        }

        months.sort_by_key(|month| month.start);

        accept(months)
    }

    fn new(label: String, year: i32, month: u8) -> BaseResult<Self> {
        //
        let month = Month::try_from(month)
            .map_err(|_| args("error-invalid-comic-archive-month"))?;

        let start_date = Date::from_calendar_date(year, month, 1)
            .map_err(|_| args("error-invalid-comic-archive-month"))?;

        let next = match month {
            //
            Month::December => (year + 1, Month::January),

            _ => (
                year,
                Month::try_from(u8::from(month) + 1)
                    .map_err(|_| args("error-invalid-comic-archive-month"))?,
            ),
        };

        let end_date = Date::from_calendar_date(next.0, next.1, 1)
            .map_err(|_| args("error-invalid-comic-archive-month"))?;

        accept(Self {
            label,
            start: PrimitiveDateTime::new(start_date, Time::MIDNIGHT)
                .assume_utc(),
            end: PrimitiveDateTime::new(end_date, Time::MIDNIGHT).assume_utc(),
        })
    }
}

fn parse_label(label: &str) -> BaseResult<(i32, u8)> {
    //
    let Some((year, month)) = label.split_once('-') else {
        return Err(args("error-invalid-comic-archive-month"));
    };

    if year.len() != 4 || month.len() != 2 {
        return Err(args("error-invalid-comic-archive-month"));
    }

    let year = year
        .parse()
        .map_err(|_| args("error-invalid-comic-archive-month"))?;

    let month = month
        .parse()
        .map_err(|_| args("error-invalid-comic-archive-month"))?;

    accept((year, month))
}

fn args(key: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(key),
    }
}
