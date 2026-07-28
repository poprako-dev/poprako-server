//! Terminology-base validation, permissions, and cascade operations.

use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_member, check_user_is_team_proofreader,
};
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::write::termbase::{TermbaseEntry, TermbaseRepl};
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

// Build an args-level error when a termbase name is empty.
fn empty_name_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-termbase-name-required"),
    }
}

// Build an args-level error for unsupported termbase scope combinations.
fn invalid_scope_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-termbase-scope"),
    }
}

// Trim a termbase name and reject empty values.
fn normalize_name(name: String) -> BaseRest<String> {
    //
    let name = name.trim().to_string();

    if name.is_empty() {
        return Err(empty_name_err());
    }

    accept(name)
}

// Normalize an optional string by trimming whitespace; empty strings become `None`.
fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        //
        let value = value.trim().to_string();

        match value.is_empty() {
            //
            true => None,

            false => Some(value),
        }
    })
}

/// Pure terminology-base construction and validation helpers.
pub struct TermbaseComplex;

impl TermbaseComplex {
    /// Normalize an optional terminology-base name filter.
    pub fn normalize_fuzzy_name(fuzzy_name: Option<String>) -> Option<String> {
        normalize_optional(fuzzy_name)
    }

    /// Build a validated terminology-base entry.
    pub fn build_entry(
        team_id: Option<String>,
        comic_id: Option<String>,
        name: String,
        description: Option<String>,
        creator_id: String,
    ) -> BaseRest<TermbaseEntry> {
        //
        match (&team_id, &comic_id) {
            //
            (Some(_), None) | (None, Some(_)) => {}

            _ => return Err(invalid_scope_err()),
        }

        let name = normalize_name(name)?;

        let description = normalize_optional(description);

        accept(TermbaseEntry {
            id: next_snowflake_id(),
            team_id,
            comic_id,
            name,
            description,
            creator_id,
        })
    }

    /// Build a validated terminology-base profile replacement.
    pub fn build_update(
        id: String,
        name: String,
        description: Option<String>,
    ) -> BaseRest<TermbaseRepl> {
        //
        let name = normalize_name(name)?;

        let description = normalize_optional(description);

        accept(TermbaseRepl {
            id,
            name,
            description,
        })
    }

    /// Delete one terminology base and all of its child terms.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        let termbase_info =
            GetTermbaseInfoExcluded { id }.proxy_on(proxy).await?;

        DeleteTerms {
            termbase_id: &termbase_info.id,
        }
        .proxy_on(proxy)
        .await?;

        DeleteTermbase {
            id: &termbase_info.id,
        }
        .proxy_on(proxy)
        .await?;

        accept(())
    }

    /// Delete all terminology bases directly owned by a team.
    pub async fn delete_team_cascade<P>(
        proxy: &mut P,
        team_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        let termbase_infos = ListTermbaseInfosExcluded::Team { team_id }
            .proxy_on(proxy)
            .await?;

        for termbase_info in termbase_infos {
            Self::delete_cascade(proxy, &termbase_info.id).await?;
        }

        accept(())
    }

    /// Delete all terminology bases directly owned by a comic.
    pub async fn delete_comic_cascade<P>(
        proxy: &mut P,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        let termbase_infos = ListTermbaseInfosExcluded::Comic { comic_id }
            .proxy_on(proxy)
            .await?;

        for termbase_info in termbase_infos {
            Self::delete_cascade(proxy, &termbase_info.id).await?;
        }

        accept(())
    }
}

/// Permission checks for terminology-base and terminology-entry resources.
pub struct TermbasePermComplex;

impl TermbasePermComplex {
    /// Resolve the owning team for a comic.
    pub async fn resolve_team_id_from_comic<P>(
        proxy: &mut P,
        comic_id: &str,
    ) -> BaseRest<String>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>,
    {
        let comic_info = GetComicInfo {
            id: comic_id,
            incls: &[],
        }
        .proxy_on(proxy)
        .await?;

        let workset_info = GetWorksetInfo {
            id: &comic_info.workset_id,
        }
        .proxy_on(proxy)
        .await?;

        accept(workset_info.team_id)
    }

    /// Resolve the owning team for a terminology base.
    pub async fn resolve_team_id<P>(
        proxy: &mut P,
        termbase_info: &TermbaseInfo,
    ) -> BaseRest<String>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>,
    {
        if let Some(team_id) = &termbase_info.team_id {
            return accept(team_id.clone());
        }

        let Some(comic_id) = &termbase_info.comic_id else {
            return Err(invalid_scope_err());
        };

        Self::resolve_team_id_from_comic(proxy, comic_id).await
    }

    /// Verify team membership for a terminology-base read.
    pub async fn ensure_user_can_read_team<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }

    /// Verify proofreader membership for a terminology-base write.
    pub async fn ensure_user_can_write_team<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_proofreader(proxy, user_id, team_id).await
    }

    /// Verify team membership for a terminology-base read.
    pub async fn ensure_user_can_read<P>(
        proxy: &mut P,
        user_id: &str,
        termbase_info: &TermbaseInfo,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = Self::resolve_team_id(proxy, termbase_info).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify proofreader membership for a terminology-base write.
    pub async fn ensure_user_can_write<P>(
        proxy: &mut P,
        user_id: &str,
        termbase_info: &TermbaseInfo,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = Self::resolve_team_id(proxy, termbase_info).await?;

        check_user_is_team_proofreader(proxy, user_id, &team_id).await
    }
}
