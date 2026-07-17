//! Terminology-base validation, permissions, and cascade operations.

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_member, check_user_is_team_proofreader,
};
use crate::model::termbase::{TermbaseEntry, TermbaseInfo, TermbaseInfoUpdate};
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

fn invalid_scope_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-termbase-scope"),
    }
}

fn empty_name_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-termbase-name-required"),
    }
}

fn normalize_name(name: String) -> BaseResult<String> {
    let name = name.trim().to_string();

    if name.is_empty() {
        return Err(empty_name_error());
    }

    accept(name)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();

        match value.is_empty() {
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
    ) -> BaseResult<TermbaseEntry> {
        match (&team_id, &comic_id) {
            (Some(_), None) | (None, Some(_)) => {}
            _ => return Err(invalid_scope_error()),
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
    ) -> BaseResult<TermbaseInfoUpdate> {
        let name = normalize_name(name)?;

        let description = normalize_optional(description);

        accept(TermbaseInfoUpdate {
            id,
            name,
            description,
        })
    }

    /// Delete one terminology base and all of its child terms.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseResult<()>
    where
        P: for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        let termbase_info = proxy.exec(&GetTermbaseInfoExcluded { id }).await?;

        proxy
            .exec(&DeleteTerms {
                termbase_id: &termbase_info.id,
            })
            .await?;

        proxy
            .exec(&DeleteTermbase {
                id: &termbase_info.id,
            })
            .await?;

        accept(())
    }

    /// Delete all terminology bases directly owned by a team.
    pub async fn delete_team_cascade<P>(
        proxy: &mut P,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        let termbase_infos = proxy
            .exec(&ListTermbaseInfosExcluded::Team { team_id })
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
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        let termbase_infos = proxy
            .exec(&ListTermbaseInfosExcluded::Comic { comic_id })
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
    ) -> BaseResult<String>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>,
    {
        let comic_info = proxy
            .exec(&GetComicInfo {
                id: comic_id,
                incls: &[],
            })
            .await?;

        let workset_info = proxy
            .exec(&GetWorksetInfo {
                id: &comic_info.workset_id,
            })
            .await?;

        accept(workset_info.team_id)
    }

    /// Resolve the owning team for a terminology base.
    pub async fn resolve_team_id<P>(
        proxy: &mut P,
        termbase_info: &TermbaseInfo,
    ) -> BaseResult<String>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>,
    {
        if let Some(team_id) = &termbase_info.team_id {
            return accept(team_id.clone());
        }

        let Some(comic_id) = &termbase_info.comic_id else {
            return Err(invalid_scope_error());
        };

        Self::resolve_team_id_from_comic(proxy, comic_id).await
    }

    /// Verify team membership for a terminology-base read.
    pub async fn ensure_user_can_read_team<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
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
    ) -> BaseResult<()>
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
    ) -> BaseResult<()>
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
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = Self::resolve_team_id(proxy, termbase_info).await?;

        check_user_is_team_proofreader(proxy, user_id, &team_id).await
    }
}
