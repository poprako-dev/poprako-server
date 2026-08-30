//! Total ObjDept composition.

#[cfg(test)]
mod mock_impl;

// R2 object-storage implementation.
mod r2_impl;

use std::future::Future;

use poprako_orchestra::{Level, Step};
use url::Url;

use poprako_obj_dept::actor::{ObjActor, ObjActorDesc};
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::oper::GetObjMeta;
use poprako_obj_dept::pool::{ObjPool, ObjPoolView};
use poprako_obj_dept::prom::ObjProm;
use poprako_obj_dept::rdb_impl::decode_row;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};
use poprako_obj_dept::{impl_obj_dept, objs_def, rdb_obj_prom};
use poprako_rdb_core::{RdbContext, RdbCore};

#[cfg(test)]
use crate::__impl_mock_obj_dept;

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
    /// Physical object-storage adapter.
    pool: P,
}

impl<P> ObjPoolView for NormObjView<P>
where
    P: ObjPoolView + Sync,
{
    // Generates one physical-object read URL.
    fn gen_url(
        &self,
        key: &str,
    ) -> impl Future<Output = ObjDeptRest<Url>> + Send {
        self.pool.gen_url(key)
    }

    // Checks whether one physical object exists.
    fn has(&self, key: &str) -> impl Future<Output = ObjDeptRest<bool>> + Send {
        self.pool.has(key)
    }
}

impl<'a, L, P> Step<GetObjMeta<'a, PageImage>, RdbContext<L>> for NormObjView<P>
where
    L: Level + Send,
    P: ObjPoolView + Sync,
{
    // Transaction isolation required by the metadata read.
    type Level = L;

    // Object metadata adapter error.
    type Error = ObjDeptError;

    // Reads the latest page-image metadata in the caller transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetObjMeta<'a, PageImage>,
    ) -> ObjDeptRest<Option<ObjMeta>> {
        //
        let row =
            __obj_dept_page_image::load(context.conn(), oper.id, false).await?;

        row.map_or(Ok(None), |row| decode_row(oper.id, row))
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
