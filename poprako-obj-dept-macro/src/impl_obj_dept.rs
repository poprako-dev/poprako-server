use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitStr, Result, Token, parenthesized};

// Parsed total ObjDept declaration.
struct DeptInput {
    // Total ObjDept type name.
    dept: Ident,
}

impl Parse for DeptInput {
    // Parses the total implementation and its marker manifest.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let dept = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("unexpected ObjDept implementation tokens"));
        }

        Ok(Self { dept })
    }
}

// One object entry received from the manifest callback.
struct ObjEntry {
    //
    // Compile-time object marker.
    marker: Ident,
    // Generated typed helper module.
    module: Ident,
    // Durable task topic.
    topic: LitStr,
    // Physical namespace retained by the callback contract.
    _namespace: LitStr,
}

impl Parse for ObjEntry {
    // Parses one callback manifest entry.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let content;

        parenthesized!(content in input);

        let marker = content.parse()?;

        content.parse::<Token![,]>()?;

        let module = content.parse()?;

        content.parse::<Token![,]>()?;

        let topic = content.parse()?;

        content.parse::<Token![,]>()?;

        let namespace = content.parse()?;

        Ok(Self {
            marker,
            module,
            topic,
            _namespace: namespace,
        })
    }
}

// Complete callback input used to generate ObjDept implementations.
struct ItemsInput {
    //
    // Total ObjDept type name.
    dept: Ident,
    // Registered objects.
    entries: Punctuated<ObjEntry, Token![,]>,
}

impl Parse for ItemsInput {
    // Parses the type name and all callback manifest entries.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let dept = input.parse()?;

        input.parse::<Token![;]>()?;

        let entries = input.parse_terminated(ObjEntry::parse, Token![,])?;

        Ok(Self { dept, entries })
    }
}

/// Expands the total `ObjDept` operations and actor construction.
pub fn expand(input: TokenStream) -> Result<TokenStream> {
    //
    let DeptInput { dept } = syn::parse2(input)?;

    Ok(quote! {
        macro_rules! __impl_obj_dept_callback {
            //
            ($($entry:tt)*) => {
                //
                ::poprako_obj_dept::__impl_obj_dept_items! {
                    #dept;
                    $($entry)*
                }
            };
        }

        __objs_manifest!(__impl_obj_dept_callback);
    })
}

/// Expands complete `ObjDept` items after the local manifest callback.
pub fn expand_items(input: TokenStream) -> Result<TokenStream> {
    //
    let ItemsInput { dept, entries } = syn::parse2(input)?;

    let helper =
        format_ident!("__obj_dept_{}", to_snake_case(&dept.to_string()),);

    let op_impls = entries
        .iter()
        .map(|entry| expand_op_impl(&dept, &entry.marker, &helper));

    let dispatch_arms = entries.iter().map(|entry| {
        //
        let module = &entry.module;

        let topic = &entry.topic;

        quote! {
            #topic => ::poprako_obj_dept::__obj_handle!(
                core,
                pool,
                task,
                #module,
            ),
        }
    });

    Ok(quote! {
        #[doc(hidden)]
        mod #helper {
            //
            use ::diesel::sql_types::{BigInt, Text};

            use ::diesel_async::RunQueryDsl as _;

            ::diesel::define_sql_function! {
                fn hashtextextended(value: Text, seed: BigInt) -> BigInt;
            }

            ::diesel::define_sql_function! {
                //
                fn pg_try_advisory_xact_lock(
                    key: BigInt,
                ) -> ::diesel::sql_types::Bool;
            }

            pub async fn lock_obj(
                conn: &mut ::poprako_rdb_core::RdbConn,
                topic: &str,
                id: &str,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                //
                let lock_key = format!("{}:{}", topic, id);

                let is_locked = ::diesel::select(
                    pg_try_advisory_xact_lock(
                        hashtextextended(lock_key, 0),
                    ),
                )
                .get_result::<bool>(conn)
                .await
                .map_err(
                    ::poprako_obj_dept::rdb_impl::diesel_err,
                )?;

                if is_locked {
                    return Ok(());
                }

                Err(
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object is busy".into(),
                    },
                )
            }
        }

        impl<P, M> #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync,
        {
            async fn dispatch(
                core: ::poprako_rdb_core::RdbCore,
                pool: P,
                task: ::poprako_obj_dept::model::task::ObjPromTask,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::poprako_obj_dept::model::task::ObjTaskAction,
            > {
                match task.topic.as_str() {
                    #(#dispatch_arms)*
                    _ => Ok(
                        ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                            message: "unknown object topic".into(),
                        },
                    ),
                }
            }
        }

        #(#op_impls)*
    })
}

// Expands Orchestra operations for one object marker.
macro_rules! expand_op_impl_tokens {
    ($dept:expr, $obj:expr, $helper:expr) => {{
        let dept = $dept;

        let obj = $obj;

        let helper = $helper;

        let obj_mod =
            format_ident!("__obj_dept_{}", to_snake_case(&obj.to_string()),);

        quote! {
        impl<'a, P, M> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::GetObjMeta<'a, #obj>,
        > for #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GetObjMeta<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::model::meta::ObjMeta>,
            > {
                let mut conn = self.core().get().await.map_err(
                    ::poprako_obj_dept::rdb_impl::rdb_err,
                )?;

                let row = #obj_mod::load(&mut conn, oper.id, false).await?;

                row.map_or(Ok(None), |row| {
                    ::poprako_obj_dept::rdb_impl::decode_row(oper.id, row)
                })
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::GetObjMeta<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::GetObjMeta<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::model::meta::ObjMeta>,
            > {
                let row = #obj_mod::load(context.conn(), oper.id, false)
                    .await?;

                row.map_or(Ok(None), |row| {
                    ::poprako_obj_dept::rdb_impl::decode_row(oper.id, row)
                })
            }
        }

        impl<'a, P, M> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::GenObjUrl<'a, #obj>,
        > for #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GenObjUrl<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<Option<::url::Url>> {
                use ::poprako_obj_dept::pool::ObjPool as _;
                let mut conn = self.core().get().await.map_err(
                    ::poprako_obj_dept::rdb_impl::rdb_err,
                )?;

                let row = #obj_mod::load(&mut conn, oper.id, false).await?;

                let meta = row.map(|row| {
                    ::poprako_obj_dept::rdb_impl::decode_row(oper.id, row)
                }).transpose()?.flatten();

                let Some(meta) = meta else {
                    return Ok(None);
                };

                if !meta.f_is_uploaded {
                    return Ok(None);
                }

                let key = meta.key.encode(#obj_mod::NAMESPACE);

                self.pool().gen_url(&key).await.map(Some)
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::GenObjSlot<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync
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
                use ::poprako_obj_dept::pool::ObjPool as _;
                use ::poprako_obj_dept::prom::ObjPromDefer as _;
                #helper::lock_obj(
                    context.conn(),
                    #obj_mod::TOPIC,
                    oper.spec.id,
                )
                .await?;

                let prev = #obj_mod::load(
                    context.conn(),
                    oper.spec.id,
                    true,
                )
                .await?;

                let version = ::poprako_obj_dept::rdb_impl::next_version(
                    oper.spec.id,
                    prev.as_ref(),
                )?;

                let key = ::poprako_obj_dept::key::ObjKey {
                    id: oper.spec.id.to_owned(),
                    version,
                };

                let physical_key = key.encode(#obj_mod::NAMESPACE);

                let pool_slot = self.pool().gen_slot(
                    &physical_key,
                    oper.spec.content_type,
                    oper.spec.byte_len,
                ).await?;

                let write = ::poprako_obj_dept::rdb_impl::ObjRdbWrite {
                    id: oper.spec.id,
                    version,
                    hash: oper.spec.hash,
                    ext: oper.spec.ext,
                };

                #obj_mod::write(context.conn(), write).await?;

                if let Some(prev_key) =
                    ::poprako_obj_dept::rdb_impl::active_key(
                        oper.spec.id,
                        prev.as_ref(),
                    )?
                {
                    self.prom()
                        .defer_delete(context, #obj_mod::TOPIC, &prev_key)
                        .await?;
                }

                self.prom()
                    .defer_check(
                        context,
                        #obj_mod::TOPIC,
                        &key,
                        pool_slot.expires_at,
                    )
                    .await?;

                Ok(::poprako_obj_dept::model::slot::ObjSlot {
                    key,
                    url: pool_slot.url,
                    headers: pool_slot.headers,
                    expires_at: pool_slot.expires_at,
                })
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::DelObjs<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #dept<P, M>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjProm + ::core::marker::Sync
                + ::poprako_obj_dept::prom::ObjPromDefer<
                    ::poprako_rdb_core::RdbContext<L>,
                >,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::DelObjs<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                use ::poprako_obj_dept::prom::ObjPromDefer as _;
                let ids = match oper {
                    ::poprako_obj_dept::oper::DelObjs::Detach { ids, .. }
                    | ::poprako_obj_dept::oper::DelObjs::Remove { ids, .. } => ids,
                };
                let mut obj_ids = ids.to_vec();

                obj_ids.sort_unstable();

                obj_ids.dedup();

                for obj_id in obj_ids {
                    #helper::lock_obj(
                        context.conn(),
                        #obj_mod::TOPIC,
                        &obj_id,
                    )
                    .await?;

                    let row = #obj_mod::load(
                        context.conn(),
                        &obj_id,
                        true,
                    )
                    .await?;

                    if let Some(key) =
                        ::poprako_obj_dept::rdb_impl::active_key(
                            &obj_id,
                            row.as_ref(),
                        )?
                    {
                        self.prom()
                            .defer_delete(context, #obj_mod::TOPIC, &key)
                            .await?;
                    }

                    match oper {
                        ::poprako_obj_dept::oper::DelObjs::Detach { .. } => {
                            #obj_mod::detach(context.conn(), &obj_id).await?;
                        }

                        ::poprako_obj_dept::oper::DelObjs::Remove { .. } => {
                            #obj_mod::remove(context.conn(), &obj_id).await?;
                        }
                    }
                }

                Ok(())
            }
        }
        }
    }};
}

// Expands Orchestra operations for one object marker.
fn expand_op_impl(dept: &Ident, obj: &Ident, helper: &Ident) -> TokenStream {
    expand_op_impl_tokens!(dept, obj, helper)
}

// Converts a Rust type name into its generated module suffix.
fn to_snake_case(name: &str) -> String {
    //
    let mut snake_case = String::with_capacity(name.len());

    let mut chars = name.chars().peekable();

    let mut prev = None;

    while let Some(current) = chars.next() {
        //
        let next = chars.peek().copied();

        let follows_lowercase = prev.is_some_and(|character: char| {
            character.is_lowercase() || character.is_ascii_digit()
        });

        let f_word_boundary = current.is_uppercase()
            && !snake_case.is_empty()
            && (follows_lowercase || next.is_some_and(char::is_lowercase));

        if f_word_boundary {
            snake_case.push('_');
        }

        snake_case.extend(current.to_lowercase());

        prev = Some(current);
    }

    snake_case
}
