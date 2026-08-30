//! Total ObjDept composition.

#[cfg(test)]
mod mock_impl;

// R2 object-storage implementation.
mod r2_impl;

use poprako_obj_dept::actor::{ObjActor, ObjActorDesc};
use poprako_obj_dept::pool::{ObjPool, ObjPoolView};
use poprako_obj_dept::prom::ObjProm;
use poprako_obj_dept::{impl_obj_dept, objs_def, rdb_obj_prom};
use poprako_rdb_core::RdbCore;

#[cfg(test)]
use crate::implement_mock_obj_dept;

use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part_impl::obj_dept::r2_impl::R2ObjPool;
use crate::part_impl::repo::rdb_impl::schema::{
    t_comic_cover, t_obj_prom_task, t_page_image, t_team_avatar, t_user_avatar,
};

rdb_obj_prom! {
    RdbObjProm {
        table: t_obj_prom_task,
    }
}

objs_def! {
    PageImage {
        table: t_page_image,
        topic: "page_image",
        namespace: "page_image",
        url_profile: ImageThumbnail,
    },
    UserAvatar {
        table: t_user_avatar,
        topic: "user_avatar",
        namespace: "user_avatar",
        url_profile: ImageThumbnail,
    },
    TeamAvatar {
        table: t_team_avatar,
        topic: "team_avatar",
        namespace: "team_avatar",
        url_profile: ImageThumbnail,
    },
    ComicCover {
        table: t_comic_cover,
        topic: "comic_cover",
        namespace: "comic_cover",
        url_profile: ImageThumbnail,
    },
}

/// Total object department composed from storage and durable-task adapters.
pub struct NormObjDept<P = R2ObjPool, M = RdbObjProm> {
    //
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: P,
    /// Durable object-task adapter.
    prom: M,
    /// Control descriptor for the single actor.
    actor_desc: ObjActorDesc,
}

/// Read-only projection of object metadata and physical storage.
#[derive(Clone)]
pub struct NormObjView<P> {
    //
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: P,
}

impl<P> NormObjView<P>
where
    P: ObjPoolView,
{
    // Returns the shared relational database core.
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    // Returns the physical object-storage adapter.
    const fn pool(&self) -> &P {
        &self.pool
    }
}

impl<P, M> NormObjDept<P, M>
where
    P: ObjPool + Clone + Send + Sync + 'static,
    M: ObjProm + Clone + Send + Sync + 'static,
{
    /// Cancels the actor and waits for it to finish.
    pub async fn close(&self) {
        //
        self.actor_desc.cancel();

        self.actor_desc.join().await;
    }

    /// Returns a read-only projection without the durable object-task adapter.
    pub fn view(&self) -> NormObjView<P> {
        //
        NormObjView {
            core: self.core.clone(),
            pool: self.pool.clone(),
        }
    }

    // Creates the total department and starts its single actor.
    fn new(core: RdbCore, pool: P, prom: M) -> Self {
        //
        let actor_core = core.clone();

        let actor_pool = pool.clone();

        let actor_desc = ObjActor::new(prom.clone(), move |task| {
            //
            let core = actor_core.clone();

            let pool = actor_pool.clone();

            async move { Self::dispatch(core, pool, task).await }
        });

        Self {
            core,
            pool,
            prom,
            actor_desc,
        }
    }
}

impl<P, M> NormObjDept<P, M>
where
    P: ObjPool,
    M: ObjProm,
{
    // Returns the shared relational database core.
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    // Returns the physical object-storage adapter.
    const fn pool(&self) -> &P {
        &self.pool
    }

    // Returns the durable object-task adapter.
    const fn prom(&self) -> &M {
        &self.prom
    }
}

impl<P, M> Clone for NormObjDept<P, M>
where
    P: Clone,
    M: Clone,
{
    // Clones the shared object department handle.
    fn clone(&self) -> Self {
        //
        Self {
            core: self.core.clone(),
            pool: self.pool.clone(),
            prom: self.prom.clone(),
            actor_desc: self.actor_desc.clone(),
        }
    }
}

impl_obj_dept! {
    dept: NormObjDept,
    view: NormObjView,
}

// Expands test adapters from the object manifest.
#[cfg(test)]
macro_rules! implement_mock_obj_dept_from_manifest {
    ($(($marker:ident, $module:ident, $topic:literal, $namespace:literal, $url_profile:ident),)*) => {
        $(implement_mock_obj_dept!($marker, $topic, $namespace, $url_profile);)*
    };
}

#[cfg(test)]
for_each_obj!(implement_mock_obj_dept_from_manifest);

/// Builds the production `ObjDept` without exposing its actor-side adapter.
///
/// # Errors
///
/// Returns an error when the R2 object-storage configuration is unavailable.
pub fn new_obj_dept(
    core: RdbCore,
) -> anyhow::Result<NormObjDept<R2ObjPool, RdbObjProm>> {
    //
    let pool = R2ObjPool::from_env()?;

    let prom = RdbObjProm::new(core.clone());

    Ok(NormObjDept::new(core, pool, prom))
}
