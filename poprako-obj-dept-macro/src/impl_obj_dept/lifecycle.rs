use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::obj_dept_entry::ObjEntry;

/// Generates batch-first reservation and cleanup operations.
pub fn expand(dept: &Ident, entry: &ObjEntry) -> TokenStream {
    //
    let obj = entry.marker();

    let obj_module = entry.module();

    let clear_objs_step = cleanup_step(
        dept,
        obj,
        obj_module,
        &quote::format_ident!("ClearObjs"),
        &quote::format_ident!("detach_many"),
    );

    let delete_objs_step = cleanup_step(
        dept,
        obj,
        obj_module,
        &quote::format_ident!("DeleteObjs"),
        &quote::format_ident!("remove_many"),
    );

    let single_alloc = single_alloc(obj, dept);

    let finish_allocs = finish_allocs();

    let defer_lifecycle_tasks = defer_lifecycle_tasks(obj_module);

    let persist_allocs = persist_allocs(obj, obj_module);

    let plan_allocs = plan_allocs(obj);

    let prepare_allocs = prepare_allocs(obj, obj_module);

    let upload_run = upload_run(obj, dept, obj_module);

    let upload_step = upload_step(obj, dept, obj_module);

    quote! {
        #upload_run

        #upload_step

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::GenObjSlots<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm
                + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjDeptPromDefer<
                    ::poprako_rdb_core::RdbContext<L>,
                >,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::GenObjSlots<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::alloc::ObjAlloc,
                >,
            > {
                use ::futures_util::future::try_join_all;
                use ::poprako_obj_dept::pool::ObjDeptPool as _;
                use ::poprako_obj_dept::prom::ObjDeptPromDefer as _;

                #prepare_allocs

                #plan_allocs

                #persist_allocs

                #defer_lifecycle_tasks

                #finish_allocs

                Ok(allocs)
            }
        }

        #single_alloc

        #clear_objs_step

        #delete_objs_step
    }
}

// Generates a transaction-scoped cleanup step for an object operation.
fn cleanup_step(
    dept: &Ident,
    obj: &Ident,
    obj_module: &Ident,
    operation: &Ident,
    persist: &Ident,
) -> TokenStream {
    //
    quote! {
        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::#operation<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm
                + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjDeptPromDefer<
                    ::poprako_rdb_core::RdbContext<L>,
                >,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::#operation<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                use ::poprako_obj_dept::prom::ObjDeptPromDefer as _;

                let mut ids = oper
                    .ids
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();

                ids.sort_unstable();
                ids.dedup();

                let records = #obj_module::load_many_for_update(
                    context.conn(),
                    &ids,
                )
                .await?;
                let delete_keys = records
                    .into_iter()
                    .map(|entry| {
                        ::poprako_obj_dept::rdb_impl::active_key::<#obj>(
                            &entry.id,
                            Some(&entry.record),
                        )
                    })
                    .collect::<::poprako_obj_dept::rest::ObjDeptRest<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                self.prom()
                    .defer_deletes(context, #obj_module::TOPIC, &delete_keys)
                    .await?;

                #obj_module::#persist(context.conn(), &ids).await?;

                Ok(())
            }
        }
    }
}

// Generates single reservation.
fn single_alloc(obj: &Ident, dept: &Ident) -> TokenStream {
    //
    quote! {
        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::GenObjSlot<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm
                + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjDeptPromDefer<
                    ::poprako_rdb_core::RdbContext<L>,
                >,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::GenObjSlot<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::poprako_obj_dept::model::alloc::ObjAlloc,
            > {
                use ::poprako_orchestra::Step as _;

                let batch_oper =
                    ::poprako_obj_dept::oper::GenObjSlots::<#obj>::new(
                        ::std::slice::from_ref(oper.spec),
                    );
                let mut allocs = self.step(context, &batch_oper).await?;

                let id = <#obj as ::poprako_obj_dept::key::KeyMap>::id(
                    &oper.spec.dom,
                );

                allocs.remove(id).ok_or_else(|| {
                    ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                        message: "object allocation result is missing".into(),
                    }
                })
            }
        }
    }
}

// Generates finish allocations.
fn finish_allocs() -> TokenStream {
    //
    quote! {
        //
        for (_, key, _, _) in planned {
            //
            let pool_slot = pool_slots
                .remove(key.id.as_str())
                .ok_or_else(|| {
                    ::poprako_obj_dept::rest::ObjDeptError::
                        Unrecoverable {
                        message: "generated object slot is missing"
                            .into(),
                    }
                })?;

            allocs.insert(
                key.id.clone(),
                ::poprako_obj_dept::model::alloc::ObjAlloc::Upload {
                    slot: ::poprako_obj_dept::model::slot::ObjSlot {
                        key,
                        url: pool_slot.url,
                        headers: pool_slot.headers,
                        expires_at: pool_slot.expires_at,
                    },
                },
            );
        }
    }
}

// Generates defer lifecycle tasks.
fn defer_lifecycle_tasks(obj_module: &Ident) -> TokenStream {
    //
    quote! {
        //
        let delete_keys = planned
            .iter_mut()
            .filter_map(|(_, _, previous_key, _)| previous_key.take())
            .collect::<Vec<_>>();

        self.prom()
            .defer_deletes(context, #obj_module::TOPIC, &delete_keys)
            .await?;

        let checks = planned
            .iter()
            .map(|(_, key, _, _)| {
                //
                let pool_slot = pool_slots
                    .get(key.id.as_str())
                    .ok_or_else(|| {
                        //
                        ::poprako_obj_dept::rest::ObjDeptError::
                            Unrecoverable {
                            message:
                                "generated object slot is \
                                    missing".into(),
                        }
                    })?;

                Ok(::poprako_obj_dept::prom::ObjDeptPromCheck::new(
                    key.clone(),
                    pool_slot.expires_at,
                ))
            })
            .collect::<::poprako_obj_dept::rest::
                ObjDeptRest<Vec<_>>>()?;

        self.prom()
            .defer_checks(context, #obj_module::TOPIC, &checks)
            .await?;
    }
}

// Generates persist reservations.
fn persist_allocs(obj: &Ident, obj_module: &Ident) -> TokenStream {
    //
    quote! {
        //
        let mut pool_slots = ::std::collections::HashMap::new();

        for chunk in planned.chunks(20) {
            //
            let pool_futures = chunk.iter().map(
                |(spec, key, _, _)| async move {
                    //
                    let pool_slot = self
                        .pool()
                        .gen_slot(
                            &key.image,
                            spec.content_type,
                            spec.byte_len,
                        )
                        .await?;

                    Ok((key.id.clone(), pool_slot))
                },
            );

            pool_slots.extend(try_join_all(pool_futures).await?);
        }
        let writes = planned
            .iter()
            .filter_map(|(spec, key, _, should_write)| {
                //
                should_write.then(|| {
                    //
                    ::poprako_obj_dept::rdb_impl::ObjRdbWrite {
                        //
                        id: <#obj as
                            ::poprako_obj_dept::key::KeyMap>::id(
                            &spec.dom,
                        ),
                        ver: key.ver,
                        key: &key.image,
                        hash: spec.hash,
                        ext: <#obj as
                            ::poprako_obj_dept::key::KeyMap>::ext(
                            &spec.dom,
                        ),
                    }
                })
            })
            .collect::<Vec<_>>();

        if !writes.is_empty() {
            //
            #obj_module::write_many(context.conn(), &writes).await?;
        }
    }
}

// Generates plan reservations.
fn plan_allocs(obj: &Ident) -> TokenStream {
    //
    quote! {
        //
        for spec in &specs {
            //
            let id =
                <#obj as
                    ::poprako_obj_dept::key::KeyMap>::id(&spec.dom);

            let previous = previous_records.remove(id);

            let current_meta = previous
                .clone()
                .map(|record| {
                    //
                    ::poprako_obj_dept::rdb_impl::decode_record::<#obj>(
                        id,
                        record,
                    )
                })
                .transpose()?
                .flatten();

            let ext =
                <#obj as ::poprako_obj_dept::key::KeyMap>::ext(
                    &spec.dom,
                );

            let matching_meta = current_meta
                .as_ref()
                .filter(|meta| {
                    //
                    meta.hash.as_slice() == spec.hash && meta.ext == ext
                });

            if let Some(meta) = matching_meta.filter(|meta|
                meta.is_avail) {
                //
                allocs.insert(
                    id.to_owned(),
                    ::poprako_obj_dept::model::alloc::ObjAlloc::Avail {
                        key: meta.key.clone(),
                    },
                );

                continue;
            }

            if let Some(meta) = matching_meta {
                //
                planned.push((spec, meta.key.clone(), None, false));

                continue;
            }

            let ver = ::poprako_obj_dept::rdb_impl::next_ver(
                id,
                previous.as_ref(),
            )?;

            let previous_key =
                ::poprako_obj_dept::rdb_impl::active_key::<#obj>(
                    id,
                    previous.as_ref(),
                )?;

            let image = <#obj as
                ::poprako_obj_dept::key::KeyMap>::forward(
                &spec.dom,
                ver,
            );

            let key = ::poprako_obj_dept::key::ObjKey {
                id: id.to_owned(),
                ver,
                image,
            };

            planned.push((spec, key, previous_key, true));
        }
    }
}

// Generates prepare reservations.
fn prepare_allocs(obj: &Ident, obj_module: &Ident) -> TokenStream {
    //
    quote! {

        let mut specs = oper.specs.iter().collect::<Vec<_>>();

        specs.sort_unstable_by_key(|spec| {
            <#obj as ::poprako_obj_dept::key::KeyMap>::id(&spec.dom)
        });

        if specs.windows(2).any(|pair| {
            <#obj as ::poprako_obj_dept::key::KeyMap>::id(&pair[0].dom)
                == <#obj as ::poprako_obj_dept::key::KeyMap>::id(
                    &pair[1].dom,
                )
        }) {
            //
            return Err(
                ::poprako_obj_dept::rest::ObjDeptError::
                    invalid("duplicate object slot id".into()),
            );
        }

        let ids = specs
            .iter()
            .map(|spec| {
                <#obj as ::poprako_obj_dept::key::KeyMap>::id(&spec.dom)
            })
            .collect::<Vec<_>>();

        let records =
            #obj_module::ensure_anchors(context.conn(), &ids).await?;

        if records.len() != ids.len() {
            //
            return Err(
                ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                    message:
                        "object anchor upsert returned incomplete state"
                        .into(),
                },
            );
        }

        let mut previous_records = records
            .into_iter()
            .map(|entry| (entry.id, entry.record))
            .collect::<::std::collections::HashMap<_, _>>();
        let mut planned = Vec::with_capacity(specs.len());

        let mut allocs = ::std::collections::HashMap::new();
    }
}

// Generates mark uploaded run.
fn upload_run(obj: &Ident, dept: &Ident, obj_module: &Ident) -> TokenStream {
    //
    quote! {
        impl<'a, P, M> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::MarkObjUploaded<'a, #obj>,
        > for #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                let mut conn = self.core().get().await.map_err(
                    ::poprako_obj_dept::rdb_impl::rdb_error,
                )?;

                // SAFETY: This endpoint records the client's declaration for
                // the exact current generation without synchronous remote I/O
                // or content-hash verification. The delayed Check task remains
                // responsible for reconciling remote presence.
                let updated = #obj_module::mark_uploaded(
                    &mut conn,
                    &oper.key.id,
                    oper.key.ver,
                )
                .await?;

                match updated {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                            message:
                                "object upload mark changed multiple \
                                    records".into(),
                        },
                    ),
                }
            }
        }
    }
}

// Generates mark uploaded step.
fn upload_step(obj: &Ident, dept: &Ident, obj_module: &Ident) -> TokenStream {
    //
    quote! {
        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::MarkObjUploaded<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm + ::core::marker::Sync,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                // SAFETY: This endpoint records the client's declaration for
                // the exact current generation without synchronous remote I/O
                // or content-hash verification. The delayed Check task remains
                // responsible for reconciling remote presence.
                let updated = #obj_module::mark_uploaded(
                    context.conn(),
                    &oper.key.id,
                    oper.key.ver,
                )
                .await?;

                match updated {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                            message:
                                "object upload mark changed multiple \
                                    records".into(),
                        },
                    ),
                }
            }
        }
    }
}
