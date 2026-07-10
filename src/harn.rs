use std::marker::PhantomData;
use std::sync::Arc;

use poprako_transactional::drive::Drive;

use crate::part::auth::TokenAuth;
use crate::part::effect::EffectDevelop;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::repo::announcement::{
    AnnouncementRepo, AnnouncementRepoTransactional,
};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::comment::{CommentRepo, CommentRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::system_mail::{
    SystemMailRepo, SystemMailRepoTransactional,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Central application harness that wires together all port implementations.
///
/// Provides accessors to each subsystem (drive, repo, prom, auth, image_pool,
/// develop) and is designed to be cheaply cloned via `Arc<HarnInner>`.
pub struct Harn<C, D, R, P, A, I, V> {
    inner: Arc<HarnInner<C, D, R, P, A, I, V>>,
}

impl<C, D, R, P, A, I, V> Clone for Harn<C, D, R, P, A, I, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Inner, non-cloneable state shared across all `Harn` clones via `Arc`.
struct HarnInner<C, D, R, P, A, I, V> {
    drive: D,
    repo: R,
    prom: P,
    auth: A,
    image_pool: I,
    develop: V,

    _p: PhantomData<C>,
}

impl<C, D, R, P, A, I, V> Harn<C, D, R, P, A, I, V>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional
        + AnnouncementRepo<C>
        + AssignmentRepo<C>
        + AssignmentInvitationRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + CommentRepo<C>
        + MemberRepo<C>
        + MemberInvitationRepo<C>
        + PageRepo<C>
        + SystemMailRepo<C>
        + TeamRepo<C>
        + UnitRepo<C>
        + UserRepo<C>
        + WorksetRepo<C>,
    <R as DeriveTransactional>::Transactional:
        AnnouncementRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + AssignmentInvitationRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + ComicRepoTransactional<C>
            + CommentRepoTransactional<C>
            + MemberRepoTransactional<C>
            + MemberInvitationRepoTransactional<C>
            + PageRepoTransactional<C>
            + SystemMailRepoTransactional<C>
            + TeamRepoTransactional<C>
            + UnitRepoTransactional<C>
            + UserRepoTransactional<C>
            + WorksetRepoTransactional<C>,
    P: Prom<C>,
    A: TokenAuth,
    I: ImagePool,
    V: EffectDevelop,
{
    pub fn new(
        drive: D,
        repo: R,
        prom: P,
        auth: A,
        image_pool: I,
        develop: V,
    ) -> Self {
        Self {
            inner: Arc::new(HarnInner {
                drive,
                repo,
                prom,
                auth,
                image_pool,
                develop,
                _p: PhantomData,
            }),
        }
    }

    pub fn drive(&self) -> &D {
        &self.inner.drive
    }

    pub fn repo(&self) -> &R {
        &self.inner.repo
    }

    pub fn prom(&self) -> &P {
        &self.inner.prom
    }

    pub fn auth(&self) -> &A {
        &self.inner.auth
    }

    pub fn image_pool(&self) -> &I {
        &self.inner.image_pool
    }

    pub fn develop(&self) -> &V {
        &self.inner.develop
    }
}
