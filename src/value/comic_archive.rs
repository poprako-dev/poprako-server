//! Calendar-month values used by comic archive export.

/// Archived chapter workflow-record payload values.
pub mod workflow_record;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

use poprako_util::i18n::{trl, trl_kv};

use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::chapter_workflow_record::ChapterWorkflowRecordKind;
use crate::value::comic_archive::workflow_record::ArchivedChapterWorkflowRecordDetail;

/// Maximum number of month slots accepted by one export request.
pub const MAX_EXPORT_MONTHS: usize = 12;

/// Complete immutable comic payload serialized once when archiving.
#[derive(Serialize)]
pub struct ArchivedComicPayload<'a> {
    //
    /// Original database identifier of the comic before archiving.
    pub source_comic_id: &'a str,
    /// Workset the comic belonged to at archiving time.
    pub workset: ArchivedWorksetPayload<'a>,
    /// Display ordering index of the comic within its workset.
    pub index: usize,
    /// Title of the comic as displayed to users.
    pub title: &'a str,
    /// Author name associated with the comic.
    pub author: &'a str,
    /// Optional longer description of the comic's content.
    pub description: Option<&'a str>,
    /// Total number of chapters archived with this comic.
    pub chapter_count: usize,
    /// The next sequential index assigned to a new chapter at archiving time.
    pub chapter_next_index: usize,
    /// Identifier of the user who created this comic.
    pub creator_id: &'a str,
    /// Unix timestamp of the most recent activity on this comic.
    pub last_active_at: i64,
    /// Unix timestamp of when the comic was created.
    pub created_at: i64,
    /// Unix timestamp of when the comic was last modified.
    pub updated_at: i64,
    /// Archived payloads for every chapter in this comic.
    pub chapters: Vec<ArchivedChapterPayload<'a>>,
}

/// Immutable workset payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedWorksetPayload<'a> {
    //
    /// Original database identifier of the workset.
    pub id: &'a str,
    /// Identifier of the team that owns this workset.
    pub team_id: &'a str,
    /// Display ordering index of the workset within the team.
    pub index: usize,
    /// Human-readable name of the workset.
    pub name: &'a str,
    /// Optional description of the workset's purpose or scope.
    pub description: Option<&'a str>,
    /// Number of comics the workset contained at archiving time.
    pub comic_count: usize,
    /// The next sequential index assigned to a new comic at archiving time.
    pub comic_next_index: usize,
    /// Unix timestamp of when the workset was created.
    pub created_at: i64,
    /// Unix timestamp of when the workset was last modified.
    pub updated_at: i64,
}

/// Immutable chapter payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedChapterPayload<'a> {
    //
    /// Original database identifier of the chapter before archiving.
    pub source_chapter_id: &'a str,
    /// Whether the chapter was pinned at the top of its comic.
    pub is_pinned: bool,
    /// Display ordering index of the chapter within its comic.
    pub index: usize,
    /// Subtitle or volume label displayed for this chapter.
    pub subtitle: &'a str,
    /// Total number of pages in this chapter at archiving time.
    pub page_count: usize,
    /// Total number of translation units across all pages.
    pub total_unit_count: usize,
    /// Number of units that have been translated.
    pub translated_unit_count: usize,
    /// Number of units that have been proofread.
    pub proofread_unit_count: usize,
    /// Bitmask of workflow stages this chapter has entered.
    pub stages: u32,
    /// Identifier of the user who created this chapter.
    pub creator_id: &'a str,
    /// Unix timestamp of when the chapter was created.
    pub created_at: i64,
    /// Unix timestamp of when the chapter was last modified.
    pub updated_at: i64,
    /// Archived payloads for all assignments on this chapter.
    pub assignments: Vec<ArchivedAssignmentPayload<'a>>,
    /// Immutable workflow records without language-specific rendered text.
    pub workflow_records: Vec<ArchivedChapterWorkflowRecordPayload<'a>>,
    /// Archived payloads for all pages in this chapter.
    pub pages: Vec<ArchivedPagePayload<'a>>,
}

/// Immutable workflow record payload retained inside an archived chapter.
#[derive(Serialize)]
pub struct ArchivedChapterWorkflowRecordPayload<'a> {
    //
    /// Original workflow record identifier.
    pub id: &'a str,
    /// User that caused the record, absent for system work.
    pub actor_user_id: Option<&'a str>,
    /// Stable event kind.
    pub kind: ChapterWorkflowRecordKind,
    /// Structured, language-neutral details.
    pub payload: ArchivedChapterWorkflowRecordDetail<'a>,
    /// Unix timestamp of record creation.
    pub created_at: i64,
}

/// Immutable assignment payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedAssignmentPayload<'a> {
    //
    /// Original database identifier of the assignment before archiving.
    pub source_assignment_id: &'a str,
    /// Identifier of the user assigned to this chapter.
    pub user_id: &'a str,
    /// Bitmask of assigned role perms.
    pub roles: u32,
    /// Unix timestamp of when the assignment was created.
    pub created_at: i64,
    /// Unix timestamp of when the assignment was last modified.
    pub updated_at: i64,
    /// Archived payload for the user assigned to this role.
    pub user: ArchivedUserPayload<'a>,
}

/// Immutable user payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedUserPayload<'a> {
    //
    /// Original database identifier of the user.
    pub id: &'a str,
    /// Qualified user identifier used for login and display.
    pub qid: &'a str,
    /// Display nickname chosen by the user.
    pub nickname: &'a str,
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
pub struct ArchivedPagePayload<'a> {
    //
    /// Original database identifier of the page before archiving.
    pub source_page_id: &'a str,
    /// Display ordering index of the page within its chapter.
    pub index: usize,
    /// Total number of translation units on this page.
    pub total_unit_count: usize,
    /// Number of units on this page that have been translated.
    pub translated_unit_count: usize,
    /// Number of units on this page that have been proofread.
    pub proofread_unit_count: usize,
    /// Unix timestamp of when the page was created.
    pub created_at: i64,
    /// Unix timestamp of when the page was last modified.
    pub updated_at: i64,
    /// Archived payloads for all translation units on this page.
    pub units: Vec<ArchivedUnitPayload<'a>>,
}

/// Immutable unit payload serialized into an archive entry.
#[derive(Serialize)]
pub struct ArchivedUnitPayload<'a> {
    //
    /// Original database identifier of the unit before archiving.
    pub source_unit_id: &'a str,
    /// Display ordering index of the unit within its page.
    pub index: usize,
    /// Whether this unit is a speech bubble (true) or narration box.
    pub is_bubble: bool,
    /// Whether the proofread pass has been completed for this unit.
    pub is_proofread: bool,
    /// Horizontal coordinate of the unit's bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit's bounding box on the page.
    pub y_coord: f64,
    /// Final translated text, or None if not yet translated.
    pub translated_text: Option<&'a str>,
    /// Identifier of the user who last edited the translation, or None.
    pub last_translator_id: Option<&'a str>,
    /// Proofread revision of the translated text, or None.
    pub proofread_text: Option<&'a str>,
    /// Identifier of the user who last edited the proofread text, or None.
    pub last_proofreader_id: Option<&'a str>,
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

// Keep archive month parsing and bounds calculation centralized in this model.

impl ComicArchiveMonth {
    /// Parses distinct archive month labels and resolves their UTC bounds.
    pub fn parse_retained(
        labels: Vec<String>,
        _now: OffsetDateTime,
    ) -> BaseRest<Vec<Self>> {
        //
        if labels.is_empty() || labels.len() > MAX_EXPORT_MONTHS {
            //
            let args = HashMap::from([
                ("min_count".into(), 1_usize.into()),
                ("max_count".into(), MAX_EXPORT_MONTHS.into()),
            ]);

            let err_message =
                trl_kv("error-invalid-comic-archive-month-count", &args);

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                label_count = labels.len(),
                "expected error: invalid comic archive month count",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        let mut unique_labels = HashSet::with_capacity(labels.len());

        for label in &labels {
            //
            if !unique_labels.insert(label.as_str()) {
                //
                let err_message = trl("error-duplicate-comic-archive-month");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    label = %label,
                    "expected error: duplicate comic archive month",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }
        }

        let mut months = Vec::with_capacity(labels.len());

        for label in labels {
            //

            let (year, month) = parse_label(&label)?;

            months.push(Self::new(label, year, month)?);
        }

        months.sort_by_key(|month| month.start);

        accept(months)
    }

    // Construct a month slot from validated label, year, and month components.
    fn new(label: String, year: i32, month: u8) -> BaseRest<Self> {
        //
        let month = Month::try_from(month).map_err(|_| {
            //
            let err_message = trl("error-invalid-comic-archive-month");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                label = %label,
                year,
                raw_month = month,
                "expected error: invalid comic archive month",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

        let start_date =
            Date::from_calendar_date(year, month, 1).map_err(|_| {
                //
                let err_message = trl("error-invalid-comic-archive-month");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    label = %label,
                    year,
                    month = ?month,
                    "expected error: invalid comic archive month start date",
                );

                BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                }
            })?;

        let next = match month {
            //
            Month::December => (year + 1, Month::January),

            _ => (
                year,
                Month::try_from(u8::from(month) + 1).map_err(|_| {
                    //
                    let err_message = trl("error-invalid-comic-archive-month");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        label = %label,
                        year,
                        month = ?month,
                        next_month = u8::from(month) + 1,
                        "expected error: invalid comic archive next month",
                    );

                    BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    }
                })?,
            ),
        };

        let end_date =
            Date::from_calendar_date(next.0, next.1, 1).map_err(|_| {
                //
                let err_message = trl("error-invalid-comic-archive-month");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    label = %label,
                    next_year = next.0,
                    next_month = ?next.1,
                    "expected error: invalid comic archive month end date",
                );

                BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                }
            })?;

        accept(Self {
            label,
            start: PrimitiveDateTime::new(start_date, Time::MIDNIGHT)
                .assume_utc(),
            end: PrimitiveDateTime::new(end_date, Time::MIDNIGHT).assume_utc(),
        })
    }
}

// Parse a "YYYY-MM" label string into its year and month components.
fn parse_label(label: &str) -> BaseRest<(i32, u8)> {
    //
    let Some((year, month)) = label.split_once('-') else {
        //
        let err_message = trl("error-invalid-comic-archive-month");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            label = %label,
            "expected error: comic archive month label has no separator",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    };

    if year.len() != 4 || month.len() != 2 {
        //
        let err_message = trl("error-invalid-comic-archive-month");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            label = %label,
            raw_year = %year,
            raw_month = %month,
            "expected error: comic archive month label has invalid width",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let year = year.parse().map_err(|_| {
        //
        let err_message = trl("error-invalid-comic-archive-month");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            label = %label,
            raw_year = %year,
            raw_month = %month,
            "expected error: comic archive month year is not numeric",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    let month = month.parse().map_err(|_| {
        //
        let err_message = trl("error-invalid-comic-archive-month");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            label = %label,
            year,
            raw_month = %month,
            "expected error: comic archive month number is not numeric",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    accept((year, month))
}
