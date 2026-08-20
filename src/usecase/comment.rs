//! Comment use cases — list and create team board comments.

#[cfg(test)]
// Unit tests that validate comment lifecycle and visibility constraints.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comment::{CommentComplex, CommentPermComplex};
use crate::data::instr::comment::{CreateCommentInstr, ListCommentInfosInstr};
use crate::data::val::comment::CreateCommentVal;
use crate::data::view::comment::CommentInfoView;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::comment::CommentEntry;
use crate::part::image::ImagePool;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::comment::CommentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Lists comments under a team.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListCommentInfosInstr,
) -> BaseRest<Vec<CommentInfoView>>
where
    C: Context,
    R: CommentRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    let comment_list_spec = Into::<CommentListSpec>::into(instr);

    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &comment_list_spec.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    CommentPermComplex::ensure_user_can_list_infos(&member_info)?;

    let comment_infos = ListCommentInfos {
        spec: &comment_list_spec,
    }
    .run_on(repo)
    .await?;

    let mut comment_info_vals = Vec::with_capacity(comment_infos.len());

    for comment_info in comment_infos {
        //
        comment_info_vals
            .push(CommentInfoView::from_model(image_pool, comment_info).await?);
    }

    accept(comment_info_vals)
}

/// Creates a comment under a team.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateCommentInstr,
) -> BaseRest<CreateCommentVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: CommentRepo<C> + MemberRepo<C> + Send + Sync,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    CommentPermComplex::ensure_user_can_create(&member_info)?;

    let comment_info = nucl
        .coord(async move |context| {
            //
            let comment_entry = CommentEntry {
                id: CommentComplex::gen_id(),
                team_id: instr.team_id,
                user_id: token.user_id,
                content: instr.content,
            };

            CreateComment {
                entry: &comment_entry,
            }
            .step_on(repo, context)
            .await
        })
        .await?;

    accept(CreateCommentVal {
        id: comment_info.id,
    })
}
