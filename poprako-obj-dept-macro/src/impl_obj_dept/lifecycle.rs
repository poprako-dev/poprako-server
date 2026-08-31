use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::obj_dept_entry::ObjEntry;

/// Generates batch-first reservation and cleanup operations.
#[expect(
    clippy::too_many_lines,
    reason = "one marker lifecycle shares a single batch-first generated engine"
)]
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

    quote! {
        impl<'a, P, M> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::MarkObjUploaded<'a, #obj>,
        > for #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                let mut conn = self.core().get().await.map_err(
                    ::poprako_obj_dept::rdb_impl::rdb_err,
                )?;

                // SAFETY: This endpoint records the client's declaration for
                // the exact current generation without synchronous remote I/O
                // or content-hash verification. The delayed Check task remains
                // responsible for reconciling remote presence.
                let updated = #obj_module::mark_uploaded(
                    &mut conn,
                    &oper.key.id,
                    oper.key.version,
                )
                .await?;

                match updated {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                            message: "object upload mark changed multiple rows".into(),
                        },
                    ),
                }
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::GenObjSlots<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm
                + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjPromDefer<
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
                    ::poprako_obj_dept::model::slot::ObjSlot,
                >,
            > {
                use ::futures_util::future::try_join_all;
                use ::poprako_obj_dept::pool::ObjPool as _;
                use ::poprako_obj_dept::prom::ObjPromDefer as _;

                const POOL_CONCURRENCY: usize = 20;

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
                        ::poprako_obj_dept::rest::ObjDeptError::Invalid {
                            message: "duplicate object slot id".into(),
                        },
                    );
                }

                let ids = specs
                    .iter()
                    .map(|spec| {
                        <#obj as ::poprako_obj_dept::key::KeyMap>::id(&spec.dom)
                            .to_owned()
                    })
                    .collect::<Vec<_>>();

                let rows =
                    #obj_module::ensure_anchors(context.conn(), &ids).await?;

                if rows.len() != ids.len() {
                    //
                    return Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                            message: "object anchor upsert returned incomplete state"
                                .into(),
                        },
                    );
                }

                let mut previous_rows = rows
                    .into_iter()
                    .map(|entry| (entry.id, entry.row))
                    .collect::<::std::collections::HashMap<_, _>>();
                let mut planned = Vec::with_capacity(specs.len());

                for spec in &specs {
                    //
                    let id =
                        <#obj as ::poprako_obj_dept::key::KeyMap>::id(&spec.dom);

                    let previous = previous_rows.remove(id);

                    let version = ::poprako_obj_dept::rdb_impl::next_version(
                        id,
                        previous.as_ref(),
                    )?;

                    let previous_key =
                        ::poprako_obj_dept::rdb_impl::active_key::<#obj>(
                            id,
                            previous.as_ref(),
                        )?;

                    let image = <#obj as ::poprako_obj_dept::key::KeyMap>::forward(
                        &spec.dom,
                        version,
                    );

                    let key = ::poprako_obj_dept::key::ObjKey {
                        id: id.to_owned(),
                        version,
                        image,
                    };

                    planned.push((key, previous_key));
                }

                let paired = specs.iter().zip(&planned).collect::<Vec<_>>();
                let mut pool_slots = ::std::collections::HashMap::new();

                for chunk in paired.chunks(POOL_CONCURRENCY) {
                    //
                    let pool_futures = chunk.iter().map(
                        |(spec, (key, _))| async move {
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
                let writes = specs
                    .iter()
                    .zip(&planned)
                    .map(|(spec, (key, _))| {
                        ::poprako_obj_dept::rdb_impl::ObjRdbWrite {
                            id: <#obj as ::poprako_obj_dept::key::KeyMap>::id(
                                &spec.dom,
                            ),
                            version: key.version,
                            key: &key.image,
                            hash: spec.hash,
                            ext: <#obj as ::poprako_obj_dept::key::KeyMap>::ext(
                                &spec.dom,
                            ),
                        }
                    })
                    .collect::<Vec<_>>();

                #obj_module::write_many(context.conn(), &writes).await?;

                let delete_keys = planned
                    .iter_mut()
                    .filter_map(|(_, previous_key)| previous_key.take())
                    .collect::<Vec<_>>();

                self.prom()
                    .defer_deletes(context, #obj_module::TOPIC, &delete_keys)
                    .await?;

                let checks = planned
                    .iter()
                    .map(|(key, _)| {
                        let pool_slot = pool_slots
                            .get(key.id.as_str())
                            .ok_or_else(|| {
                            ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                                message: "generated object slot is missing".into(),
                            }
                        })?;

                        Ok(::poprako_obj_dept::prom::ObjPromCheck::new(
                            key.clone(),
                            pool_slot.expires_at,
                        ))
                    })
                    .collect::<::poprako_obj_dept::rest::ObjDeptRest<Vec<_>>>()?;

                self.prom()
                    .defer_checks(context, #obj_module::TOPIC, &checks)
                    .await?;

                let mut slots = ::std::collections::HashMap::new();

                for (key, _) in planned {
                    //
                    let pool_slot = pool_slots
                        .remove(key.id.as_str())
                        .ok_or_else(|| {
                            //
                            ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                                message: "generated object slot is missing".into(),
                            }
                        })?;

                    slots.insert(
                        key.id.clone(),
                        ::poprako_obj_dept::model::slot::ObjSlot {
                            key,
                            url: pool_slot.url,
                            headers: pool_slot.headers,
                            expires_at: pool_slot.expires_at,
                        },
                    );
                }

                Ok(slots)
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::GenObjSlot<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm
                + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjPromDefer<
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
                ::poprako_obj_dept::model::slot::ObjSlot,
            > {
                use ::poprako_orchestra::Step as _;

                let batch_oper =
                    ::poprako_obj_dept::oper::GenObjSlots::<#obj>::new(
                        ::std::slice::from_ref(oper.spec),
                    );
                let mut slots = self.step(context, &batch_oper).await?;

                let id = <#obj as ::poprako_obj_dept::key::KeyMap>::id(
                    &oper.spec.dom,
                );

                slots.remove(id).ok_or_else(|| {
                    ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                        message: "generated object slot is missing".into(),
                    }
                })
            }
        }

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
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm
                + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjPromDefer<
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
                use ::poprako_obj_dept::prom::ObjPromDefer as _;

                let mut ids = oper.ids.to_vec();

                ids.sort_unstable();
                ids.dedup();

                let rows = #obj_module::load_many_for_update(
                    context.conn(),
                    &ids,
                )
                .await?;
                let delete_keys = rows
                    .into_iter()
                    .map(|entry| {
                        ::poprako_obj_dept::rdb_impl::active_key::<#obj>(
                            &entry.id,
                            Some(&entry.row),
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
