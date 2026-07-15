use std::marker::PhantomData;
use std::sync::Arc;

use poprako_orchestra::Nucl;

use crate::part::auth::TokenAuth;
use crate::part::effect::EffectDevelop;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::repo::announcement::AnnouncementRepo;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comment::CommentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::BaseError;

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
    D: Nucl<Context = C, Error = BaseError>,
    R: AnnouncementRepo<C>
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
    P: Prom<C>,
    A: TokenAuth,
    I: ImagePool + Sync,
    V: EffectDevelop + Sync,
{
    /// Builds a new `Harn` from the given port implementations.
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

    /// Returns a reference to the transaction driver.
    pub fn drive(&self) -> &D {
        &self.inner.drive
    }

    /// Returns a reference to the repository bundle.
    pub fn repo(&self) -> &R {
        &self.inner.repo
    }

    /// Returns a reference to the prom (deferred task) enqueuer.
    pub fn prom(&self) -> &P {
        &self.inner.prom
    }

    /// Returns a reference to the auth (token signer).
    pub fn auth(&self) -> &A {
        &self.inner.auth
    }

    /// Returns a reference to the image pool (upload/download URL signing).
    pub fn image_pool(&self) -> &I {
        &self.inner.image_pool
    }

    /// Returns a reference to the side-effect (event) processor.
    pub fn develop(&self) -> &V {
        &self.inner.develop
    }
}
