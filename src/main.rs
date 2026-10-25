#![recursion_limit = "512"]

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::Context as _;
use tokio::sync::Semaphore;
use tokio::task::JoinError;

use poprako_obj_dept::actor::{ObjDeptActor, ObjDeptActorDesc};
use poprako_prom::general::actor::{PromActor, PromActorDesc};
use poprako_prom::general::handler::Dispatcher;
use poprako_server::part_impl::prom::rdb_impl::delivery::RdbPromDelivery;
use poprako_server::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use poprako_server::part_impl::prom::rdb_impl::writer::RdbProm;
use poprako_server::{
    AppConfig, AsyncEffectDevelop, EffectActor, EffectActorDesc, Harn, HybNucl,
    JwtAuth, MemCache, NormObjDept, R2ObjDeptPool, RdbContext, RdbCore,
    RdbNucl, RdbObjDeptProm, RdbRepo, ReptRead, Sched, SchedDesc, Serial,
    SubtreeDeleteTask, dispatch_prom, serve,
};

// Report every failed supervisor after all joins have completed.
fn report_shutdown(
    rests: [(&str, Result<(), JoinError>); 4],
) -> anyhow::Result<()> {
    //
    rests
        .into_iter()
        .map(|(actor, rest)| {
            //
            rest.map_err(|error| {
                //
                tracing::error!(
                    actor,
                    error = ?error,
                    "background supervisor failed"
                );

                anyhow::Error::new(error)
                    .context([actor, " supervisor failed"].concat())
            })
        })
        .fold(Ok(()), Result::and)
}

// Load optional local environment before constructing adapters.
fn load_dotenv() {
    //
    if let Err(error) = dotenvy::dotenv() {
        //
        #[cfg(debug_assertions)]
        let sdk_error = tracing::field::debug(&error);

        #[cfg(not(debug_assertions))]
        let sdk_error = {
            // Consume parsing diagnostics without formatting private payloads.
            let _ = error;

            tracing::field::display("[REDACTED]")
        };

        tracing::warn!(
            operation = "load_dotenv",
            sdk_error,
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

// Construct process-wide archive admission from the validated runtime policy.
fn archive_export_admission(
    config: &AppConfig,
) -> anyhow::Result<Arc<Semaphore>> {
    //
    let capacity =
        usize::try_from(config.comic_archive_export.concurrency.get())
            .context("archive export concurrency exceeds platform capacity")?;

    Ok(Arc::new(Semaphore::new(capacity)))
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

    let config = AppConfig::from_default_file()
        .await
        .context("failed to load application configuration")?;

    let (
        rdb_core,
        auth,
        obj_dept_pool,
        prom,
        prom_repo,
        (develop, effect_recv),
    ) = (
        RdbCore::from_env()?,
        JwtAuth::from_env()?,
        R2ObjDeptPool::from_env()?,
        RdbProm::new(),
        RdbPromRepo::new(),
        AsyncEffectDevelop::new(config.effect.capacity),
    );

    let (http_addr, rept_read_nucl, serial_nucl, repo, cache, obj_dept_prom) = (
        http_addr(&config)?,
        RdbNucl::<ReptRead>::new(rdb_core.clone()),
        RdbNucl::<Serial>::new(rdb_core.clone()),
        RdbRepo::new(rdb_core.clone(), config.repo),
        MemCache::new(config.cache),
        RdbObjDeptProm::new(rdb_core.clone()),
    );

    let (obj_dept, nucl, archive_exports) = (
        NormObjDept::new(
            rdb_core.clone(),
            obj_dept_pool.clone(),
            obj_dept_prom.clone(),
        ),
        HybNucl::new(rept_read_nucl.clone(), serial_nucl),
        archive_export_admission(&config)?,
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
                Box::pin(
                    NormObjDept::<R2ObjDeptPool, RdbObjDeptProm>::dispatch(
                        rdb_core,
                        obj_dept_pool,
                        task,
                    ),
                )
                .await
            }
        }),
        Sched::new(
            vec![Box::new(SubtreeDeleteTask::new(
                rept_read_nucl,
                repo.clone(),
                obj_dept.clone(),
                config.subtree_delete,
            ))],
            config.sched,
        ),
    );

    let (
        harn,
        obj_dept_actor_desc,
        effect_actor_desc,
        prom_actor_desc,
        sched_desc,
    ) = (
        Harn::new(config, (nucl, repo, cache, obj_dept, prom, auth, develop)),
        obj_dept_actor.run_detached(),
        effect_actor.run_detached::<RdbContext<ReptRead>>(),
        prom_actor.run_detached(),
        sched.run_detached(),
    );

    let serve_rest = serve(harn, http_addr, archive_exports).await;

    let shutdown_rest = join_background(
        sched_desc,
        prom_actor_desc,
        effect_actor_desc,
        obj_dept_actor_desc,
    )
    .await;

    serve_rest.and(shutdown_rest)
}
