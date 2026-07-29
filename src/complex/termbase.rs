use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{DeleteTermbase, LockTermbaseExcluded};
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

/// Domain operations for termbase entities.
pub struct TermbaseComplex;

impl TermbaseComplex {
    /// Deletes a termbase and its child terms inside an existing transaction context.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseResult<()>
    where
        P: for<'a> Proxy<LockTermbaseExcluded<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<DeleteTerms<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>,
    {
        proxy.exec(&LockTermbaseExcluded { id }).await?;

        proxy
            .exec(&DeleteTerms::Termbase { termbase_id: id })
            .await?;

        proxy.exec(&DeleteTermbase { id }).await?;

        accept(())
    }
}

/// Permission-gate operations for termbase entities.
pub struct TermbasePermComplex;

impl TermbasePermComplex {
    /// Verifies the caller is not a translator or proofreader (must be admin) for team-scoped termbases.
    pub async fn ensure_user_can_create_team_termbase<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let member_info = proxy
            .exec(&FindMemberInfo::UserTeam { user_id, team_id })
            .await?
            .ok_or_else(perm_denied)?;

        if member_info
            .roles
            .has_any_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
        {
            return Err(perm_denied());
        }

        accept(())
    }

    /// Verifies the caller can create a comic-scoped termbase by resolving the owning team
    /// from the chapter's comic and delegating to the team-level check.
    pub async fn ensure_user_can_create_comic_termbase<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindAssignmentInfo<'a, 'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let assignment_info = proxy
            .exec(&FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls: &[AssignmentInclOpt::ChapterComicWorksetTeam],
            })
            .await?
            .ok_or_else(perm_denied)?;

        let team_id = &assignment_info
            .chapter
            .and_then(|c| c.comic)
            .and_then(|c| c.team)
            .map(|t| t.id)
            .ok_or_else(|| BaseError::Unrecoverable {
                message: "TODO".to_string(),
            })?;

        // Delegate to team level permission check.
        Self::ensure_user_can_create_team_termbase(proxy, user_id, team_id)
            .await?;

        accept(())
    }
}

fn perm_denied() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-forbidden"),
    }
}
