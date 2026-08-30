use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::obj_dept_entry::ObjEntry;

/// Generates batch-first reservation and retirement operations.
#[expect(
    clippy::too_many_lines,
    reason = "one marker lifecycle shares a single batch-first generated engine"
)]
pub fn expand(dept: &Ident, entry: &ObjEntry) -> TokenStream {
    //
    let obj = entry.marker();

    let obj_module = entry.module();

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
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome,
            > {
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
                    0 => Ok(
                        ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome::NotCurrent,
                    ),
                    1 => Ok(
                        ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome::Marked,
                    ),
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

                specs.sort_unstable_by_key(|spec| spec.id);

                if specs.windows(2).any(|pair| pair[0].id == pair[1].id) {
                    //
                    return Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Invalid {
                            message: "duplicate object slot id".into(),
                        },
                    );
                }

                let ids = specs
                    .iter()
                    .map(|spec| spec.id.to_owned())
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
                    let previous = previous_rows.remove(spec.id);

                    let version = ::poprako_obj_dept::rdb_impl::next_version(
                        spec.id,
                        previous.as_ref(),
                    )?;

                    let previous_key =
                        ::poprako_obj_dept::rdb_impl::active_key(
                            spec.id,
                            previous.as_ref(),
                    )?;

                    let key = ::poprako_obj_dept::key::ObjKey {
                        id: spec.id.to_owned(),
                        version,
                    };

                    planned.push((key, previous_key));
                }

                let paired = specs.iter().zip(&planned).collect::<Vec<_>>();
                let mut pool_slots = ::std::collections::HashMap::new();

                for chunk in paired.chunks(POOL_CONCURRENCY) {
                    let pool_futures = chunk.iter().map(
                        |(spec, (key, _))| async move {
                            let physical_key =
                                key.encode(#obj_module::NAMESPACE);
                            let pool_slot = self
                                .pool()
                                .gen_slot(
                                    &physical_key,
                                    spec.content_type,
                                    spec.byte_len,
                                )
                                .await?;

                            Ok((spec.id.to_owned(), pool_slot))
                        },
                    );

                    pool_slots.extend(try_join_all(pool_futures).await?);
                }
                let writes = specs
                    .iter()
                    .zip(&planned)
                    .map(|(spec, (key, _))| {
                        ::poprako_obj_dept::rdb_impl::ObjRdbWrite {
                            id: spec.id,
                            version: key.version,
                            hash: spec.hash,
                            ext: spec.ext,
                        }
                    })
                    .collect::<Vec<_>>();

                #obj_module::write_many(context.conn(), &writes).await?;

                let delete_keys = planned
                    .iter()
                    .filter_map(|(_, previous_key)| previous_key.clone())
                    .collect::<Vec<_>>();

                self.prom()
                    .defer_deletes(context, #obj_module::TOPIC, &delete_keys)
                    .await?;

                let checks = planned
                    .iter()
                    .map(|(key, _)| {
                        let pool_slot = pool_slots.get(&key.id).ok_or_else(|| {
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
                    let pool_slot = pool_slots.remove(&key.id).ok_or_else(|| {
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

                let specs = [*oper.spec];
                let batch_oper = ::poprako_obj_dept::oper::GenObjSlots::<#obj> {
                    specs: &specs,
                    _m: ::core::marker::PhantomData,
                };
                let mut slots = self.step(context, &batch_oper).await?;

                slots.remove(oper.spec.id).ok_or_else(|| {
                    ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                        message: "generated object slot is missing".into(),
                    }
                })
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::RetireObjs<'a, #obj>,
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
                oper: &::poprako_obj_dept::oper::RetireObjs<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                use ::poprako_obj_dept::prom::ObjPromDefer as _;

                let ids = match oper {
                    ::poprako_obj_dept::oper::RetireObjs::PreserveWatermarks { ids, .. }
                    | ::poprako_obj_dept::oper::RetireObjs::RemoveRows { ids, .. } => ids,
                };
                let mut ids = ids.to_vec();

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
                        ::poprako_obj_dept::rdb_impl::active_key(
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

                match oper {
                    //
                    ::poprako_obj_dept::oper::RetireObjs::PreserveWatermarks { .. } => {
                        #obj_module::detach_many(context.conn(), &ids).await?;
                    }

                    ::poprako_obj_dept::oper::RetireObjs::RemoveRows { .. } => {
                        #obj_module::remove_many(context.conn(), &ids).await?;
                    }
                }

                Ok(())
            }
        }
    }
}
