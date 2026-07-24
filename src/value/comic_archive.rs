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
    //
    /// Original database identifier of the comic before archiving.
    pub source_comic_id: String,
    /// Workset the comic belonged to at archiving time.
    pub workset: ArchivedWorksetPayload,
    /// Display ordering index of the comic within its workset.
    pub index: i32,
    /// Title of the comic as displayed to users.
    pub title: String,
    /// Author name associated with the comic.
    pub author: String,
    /// Optional longer description of the comic's content.
    pub description: Option<String>,
    /// Total number of chapters archived with this comic.
    pub chapter_count: i32,
    /// The next sequential index assigned to a new chapter at archiving time.
    pub chapter_next_index: i32,
    /// Identifier of the user who created this comic.
    pub creator_id: String,
    /// Unix timestamp of the most recent activity on this comic.
    pub last_active_at: i64,
    /// Unix timestamp of when the comic was created.
    pub created_at: i64,
    /// Unix timestamp of when the comic was last modified.
    pub updated_at: i64,
    /// Archived payloads for every chapter in this comic.
    pub chapters: Vec<ArchivedChapterPayload>,
}

/// Immutable workset payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedWorksetPayload {
    //
    /// Original database identifier of the workset.
    pub id: String,
    /// Identifier of the team that owns this workset.
    pub team_id: String,
    /// Display ordering index of the workset within the team.
    pub index: i32,
    /// Human-readable name of the workset.
    pub name: String,
    /// Optional description of the workset's purpose or scope.
    pub description: Option<String>,
    /// Number of comics the workset contained at archiving time.
    pub comic_count: i32,
    /// The next sequential index assigned to a new comic at archiving time.
    pub comic_next_index: i32,
    /// Unix timestamp of when the workset was created.
    pub created_at: i64,
    /// Unix timestamp of when the workset was last modified.
    pub updated_at: i64,
}

/// Immutable chapter payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedChapterPayload {
    //
    /// Original database identifier of the chapter before archiving.
    pub source_chapter_id: String,
    /// Whether the chapter was pinned at the top of its comic.
    pub is_pinned: bool,
    /// Display ordering index of the chapter within its comic.
    pub index: i32,
    /// Subtitle or volume label displayed for this chapter.
    pub subtitle: String,
    /// Total number of pages in this chapter at archiving time.
    pub page_count: i32,
    /// Total number of translation units across all pages.
    pub total_unit_count: i32,
    /// Number of units that have been translated.
    pub translated_unit_count: i32,
    /// Number of units that have been proofread.
    pub proofread_unit_count: i32,
    /// Bitmask of workflow stages this chapter has entered.
    pub stages: u32,
    /// Identifier of the user who created this chapter.
    pub creator_id: String,
    /// Unix timestamp of when the chapter was created.
    pub created_at: i64,
    /// Unix timestamp of when the chapter was last modified.
    pub updated_at: i64,
    /// Archived payloads for all assignments on this chapter.
    pub assignments: Vec<ArchivedAssignmentPayload>,
    /// Archived payloads for all pages in this chapter.
    pub pages: Vec<ArchivedPagePayload>,
}

/// Immutable assignment payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedAssignmentPayload {
    //
    /// Original database identifier of the assignment before archiving.
    pub source_assignment_id: String,
    /// Identifier of the user assigned to this chapter.
    pub user_id: String,
    /// Bitmask of assigned role permissions.
    pub roles: u32,
    /// Unix timestamp of when the assignment was created.
    pub created_at: i64,
    /// Unix timestamp of when the assignment was last modified.
    pub updated_at: i64,
    /// Archived payload for the user assigned to this role.
    pub user: ArchivedUserPayload,
}

/// Immutable user payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedUserPayload {
    //
    /// Original database identifier of the user.
    pub id: String,
    /// Qualified user identifier used for login and display.
    pub qid: String,
    /// Display nickname chosen by the user.
    pub nickname: String,
    /// Optional storage key for the user's avatar image.
    pub avatar_key: Option<String>,
    /// Whether the avatar has been uploaded to object storage.
    pub avatar_uploaded: bool,
    /// Version counter incremented each time the avatar is replaced.
    pub avatar_version: u32,
    /// Whether the user has super-administrator privileges.
    pub is_sadmin: bool,
    /// Unix timestamp of the user's most recent activity.
    pub last_active_at: i64,
    /// Unix timestamp of when the user account was created.
    pub created_at: i64,
    /// Unix timestamp of when the user profile was last modified.
    pub updated_at: i64,
}

/// Immutable page payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedPagePayload {
    //
    /// Original database identifier of the page before archiving.
    pub source_page_id: String,
    /// Display ordering index of the page within its chapter.
    pub index: i32,
    /// Total number of translation units on this page.
    pub total_unit_count: i32,
    /// Number of units on this page that have been translated.
    pub translated_unit_count: i32,
    /// Number of units on this page that have been proofread.
    pub proofread_unit_count: i32,
    /// Unix timestamp of when the page was created.
    pub created_at: i64,
    /// Unix timestamp of when the page was last modified.
    pub updated_at: i64,
    /// Archived payloads for all translation units on this page.
    pub units: Vec<ArchivedUnitPayload>,
}

/// Immutable unit payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedUnitPayload {
    //
    /// Original database identifier of the unit before archiving.
    pub source_unit_id: String,
    /// Display ordering index of the unit within its page.
    pub index: i32,
    /// Whether this unit is a speech bubble (true) or narration box.
    pub is_bubble: bool,
    /// Whether the proofread pass has been completed for this unit.
    pub is_proofread: bool,
    /// Horizontal coordinate of the unit's bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit's bounding box on the page.
    pub y_coord: f64,
    /// Final translated text, or None if not yet translated.
    pub translated_text: Option<String>,
    /// Identifier of the user who last edited the translation, or None.
    pub last_translator_id: Option<String>,
    /// Proofread revision of the translated text, or None.
    pub proofread_text: Option<String>,
    /// Identifier of the user who last edited the proofread text, or None.
    pub last_proofreader_id: Option<String>,
    /// Unix timestamp of when the unit was created.
    pub created_at: i64,
    /// Unix timestamp of when the unit was last modified.
    pub updated_at: i64,
}

/// One validated UTC calendar-month range.
pub struct ComicArchiveMonth {
    //
    /// Human-readable label in "YYYY-MM" format for this month slot.
    pub label: String,

    /// UTC timestamp at midnight on the first day of this month.
    pub start: OffsetDateTime,
    /// UTC timestamp at midnight on the first day of the following month.
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
