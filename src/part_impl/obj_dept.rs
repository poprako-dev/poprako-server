//! Total ObjDept composition.

#[cfg(test)]
mod mock_impl;

use poprako_obj_dept::actor::{ObjActor, ObjActorDesc};
use poprako_obj_dept::pool::ObjPool;
use poprako_obj_dept::prom::ObjProm;
use poprako_obj_dept::{impl_obj_dept, objs_def, rdb_obj_prom};
use poprako_rdb_core::RdbCore;

#[cfg(test)]
use crate::__impl_mock_obj_dept;

use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part_impl::obj_pool::r2_impl::R2ObjPool;
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
    },
    UserAvatar {
        table: t_user_avatar,
        topic: "user_avatar",
        namespace: "user_avatar",
    },
    TeamAvatar {
        table: t_team_avatar,
        topic: "team_avatar",
        namespace: "team_avatar",
    },
    ComicCover {
        table: t_comic_cover,
        topic: "comic_cover",
        namespace: "comic_cover",
    },
}

/// Total object department composed from storage and durable-task adapters.
pub struct NormObjDept<Pool, Prom> {
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: Pool,
    /// Durable object-task adapter.
    prom: Prom,
    /// Control descriptor for the single actor.
    actor_desc: ObjActorDesc,
}

impl<Pool, Prom> NormObjDept<Pool, Prom>
where
    Pool: ObjPool,
    Prom: ObjProm,
{
    /// Cancels the actor and waits for it to finish.
    pub async fn close(&self) {
        //
        self.actor_desc.cancel();

        self.actor_desc.join().await;
    }

    // Creates the total department and starts its single actor.
    fn new(core: RdbCore, pool: Pool, prom: Prom) -> Self {
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

    // Returns the shared relational database core.
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    // Returns the physical object-storage adapter.
    const fn pool(&self) -> &Pool {
        &self.pool
    }

    // Returns the durable object-task adapter.
    const fn prom(&self) -> &Prom {
        &self.prom
    }
}

impl<Pool, Prom> Clone for NormObjDept<Pool, Prom>
where
    Pool: Clone,
    Prom: Clone,
{
    // Clones handles without starting another actor.
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

impl_obj_dept! { NormObjDept }

#[cfg(test)]
// Expands test adapters from the same total object manifest.
macro_rules! __impl_mock_obj_dept_callback {
    ($(($marker:ident, $module:ident, $topic:literal, $namespace:literal),)*) => {
        $(__impl_mock_obj_dept!($marker, $topic, $namespace);)*
    };
}

#[cfg(test)]
__objs_manifest!(__impl_mock_obj_dept_callback);

/// Builds the production `ObjDept` without exposing its actor-side adapter.
#[must_use]
pub fn new_obj_dept(
    core: RdbCore,
    pool: R2ObjPool,
) -> NormObjDept<R2ObjPool, RdbObjProm> {
    //
    let prom = RdbObjProm::new(core.clone());

    NormObjDept::new(core, pool, prom)
}

/// Production object department type.
pub type AppObjDept = NormObjDept<R2ObjPool, RdbObjProm>;
