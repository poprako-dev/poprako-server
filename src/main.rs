#![recursion_limit = "512"]

use std::net::{SocketAddr, ToSocketAddrs};
use std::num::NonZeroUsize;

use anyhow::Context as _;
use tokio::task::JoinError;

use poprako_obj_dept::actor::{ObjDeptActor, ObjDeptActorDesc};
use poprako_prom::general::actor::{PromActor, PromActorDesc};
use poprako_prom::general::handler::Dispatcher;
use poprako_server::part_impl::prom::rdb_impl::delivery::RdbPromDelivery;
use poprako_server::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use poprako_server::part_impl::prom::rdb_impl::writer::RdbProm;
use poprako_server::{
    AppConfig, AsyncEffectDevelop, EffectActor, EffectActorDesc, Harn, HybNucl,
    HybRepo, JwtAuth, NormObjDept, R2ObjDeptPool, RdbContext, RdbCore, RdbNucl,
    RdbObjDeptProm, ReptRead, Sched, SchedConfig, SchedDesc, Serial,
    SubtreeDeleteTask, dispatch_prom,
};

// Report every failed supervisor after all joins have completed.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
fn report_shutdown(
    results: [(&str, Result<(), JoinError>); 4],
) -> anyhow::Result<()> {
    //
    results
        .into_iter()
        .map(|(actor, rest)| {
            //
            rest.map_err(|err| {
                //
                tracing::error!(
                    actor,
                    err = ?err,
                    "background supervisor failed"
                );

                anyhow::Error::new(err)
                    .context(format!("{} supervisor failed", actor))
            })
        })
        .fold(Ok(()), Result::and)
}

// Load optional local environment before constructing adapters.
fn load_dotenv() {
    //
    if let Err(err) = dotenvy::dotenv() {
        //
        tracing::warn!(
            operation = "load_dotenv",
            sdk_err = ?err,
            ".env loading failed; continuing with process environment",
        );
    }
}

// Resolve the configured HTTP listener before starting background actors.
fn http_addr(config: &AppConfig) -> anyhow::Result<SocketAddr> {
    //
    ToSocketAddrs::to_socket_addrs(&format!(
        "{}:{}",
        config.http.host, config.http.port,
    ))
    .into_iter()
    .find_map(|mut addrs| addrs.next())
    .context("no address resolved for HTTP listen address")
}

// Join every startup-owned descriptor and report all supervisor failures.
async fn join_background(
    sched_desc: SchedDesc,
    prom_actor_desc: PromActorDesc,
    effect_actor_desc: EffectActorDesc,
    obj_dept_actor_desc: ObjDeptActorDesc,
) -> anyhow::Result<()> {
    //
    let (sched_rest, prom_rest, effect_rest, obj_dept_rest) = tokio::join!(
        sched_desc.cancel_and_join(),
        prom_actor_desc.cancel_and_join(),
        effect_actor_desc.cancel_and_join(),
        obj_dept_actor_desc.cancel_and_join(),
    );

    report_shutdown([
        ("sched", sched_rest),
        ("prom", prom_rest),
        ("effect", effect_rest),
        ("obj_dept", obj_dept_rest),
    ])
}

/// Application entry point.
///
/// Constructs application ports, injects background dependencies, starts actors
/// explicitly, and serves HTTP. Runtime descriptors remain owned here and are
/// joined in dependency order on both normal exit and HTTP startup failure.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //
    poprako_server::init_log();

    load_dotenv();

    let (
        config,
        rdb_core,
        auth,
        obj_dept_pool,
        prom,
        prom_repo,
        (develop, effect_recv),
    ) = (
        AppConfig::from_default_file()
            .await
            .context("failed to load application configuration")?,
        RdbCore::from_env()?,
        JwtAuth::from_env()?,
        R2ObjDeptPool::from_env()?,
        RdbProm::new(),
        RdbPromRepo::new(),
        AsyncEffectDevelop::new(
            NonZeroUsize::new(1024).context("buf_size cannot be 0")?,
        ),
    );

    let (http_addr, rept_read_nucl, serial_nucl, repo, obj_dept_prom) = (
        http_addr(&config)?,
        RdbNucl::<ReptRead>::new(rdb_core.clone()),
        RdbNucl::<Serial>::new(rdb_core.clone()),
        HybRepo::new(rdb_core.clone()),
        RdbObjDeptProm::new(rdb_core.clone()),
    );

    let (obj_dept, nucl) = (
        NormObjDept::new(
            rdb_core.clone(),
            obj_dept_pool.clone(),
            obj_dept_prom.clone(),
        ),
        HybNucl::new(rept_read_nucl.clone(), serial_nucl),
    );

    let (prom_actor, effect_actor, obj_dept_actor, sched) = (
        PromActor::new(
            RdbPromDelivery::new(nucl.serial().clone(), prom_repo),
            Dispatcher::new(
                (
                    rept_read_nucl.clone(),
                    repo.clone(),
                    obj_dept.view(),
                    develop.clone(),
                ),
                |(nucl, repo, view, develop), payload| async move {
                    //
                    dispatch_prom((&nucl, &repo, &view, &develop), payload)
                        .await
                },
            ),
        ),
        EffectActor::new(repo.clone(), effect_recv),
        ObjDeptActor::new(obj_dept_prom, move |task| {
            //
            let (rdb_core, obj_dept_pool) =
                (rdb_core.clone(), obj_dept_pool.clone());

            async move {
                //
                NormObjDept::<R2ObjDeptPool, RdbObjDeptProm>::dispatch(
                    rdb_core,
                    obj_dept_pool,
                    task,
                )
                .await
            }
        }),
        Sched::new(
            vec![Box::new(SubtreeDeleteTask::new(
                rept_read_nucl,
                repo.clone(),
                obj_dept.clone(),
            ))],
            SchedConfig::default(),
        ),
    );

    let (
        harn,
        obj_dept_actor_desc,
        effect_actor_desc,
        prom_actor_desc,
        sched_desc,
    ) = (
        Harn::new(config, (nucl, repo, obj_dept, prom, auth, develop)),
        obj_dept_actor.run_detached(),
        effect_actor.run_detached::<RdbContext<ReptRead>>(),
        prom_actor.run_detached(),
        sched.run_detached(),
    );

    let serve_rest = poprako_server::serve(harn, http_addr).await;

    let shutdown_rest = join_background(
        sched_desc,
        prom_actor_desc,
        effect_actor_desc,
        obj_dept_actor_desc,
    )
    .await;

    serve_rest.and(shutdown_rest)
}
