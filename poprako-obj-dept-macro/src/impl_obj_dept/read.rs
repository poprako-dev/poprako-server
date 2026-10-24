use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::obj_dept_entry::ObjEntry;

/// Generates read operations for the total department and read projection.
pub fn expand(dept: &Ident, view: &Ident, entry: &ObjEntry) -> TokenStream {
    //
    let obj = entry.marker();

    let obj_module = entry.module();

    let view_reads = view_reads(obj, view, obj_module);

    let total_urls = total_urls(obj, dept);

    quote! {
        impl<'a, P, M> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
        > for #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                let mut conn = self.core().get().await.map_err(
                    ::poprako_obj_dept::rdb_impl::rdb_error,
                )?;

                let rows = #obj_module::load_many(&mut conn, oper.ids).await?;

                #obj_module::decode_many::<#obj>(rows)
            }
        }

        impl<'a, L, P, M> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
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
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                let rows = #obj_module::load_many(context.conn(), oper.ids)
                    .await?;

                #obj_module::decode_many::<#obj>(rows)
            }
        }

        #total_urls

        #view_reads

    }
}

// Generates view reads.
fn view_reads(obj: &Ident, view: &Ident, obj_module: &Ident) -> TokenStream {
    //
    quote! {
        impl<'a, P> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
        > for #view<P>
        where
            P: ::poprako_obj_dept::pool::ObjDeptPoolView + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                let mut conn = self.core().get().await.map_err(
                    ::poprako_obj_dept::rdb_impl::rdb_error,
                )?;

                let rows = #obj_module::load_many(&mut conn, oper.ids).await?;

                #obj_module::decode_many::<#obj>(rows)
            }
        }

        impl<'a, L, P> ::poprako_orchestra::Step<
            ::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
            ::poprako_rdb_core::RdbContext<L>,
        > for #view<P>
        where
            L: ::poprako_orchestra::Level + Send,
            P: ::poprako_obj_dept::pool::ObjDeptPoolView + ::core::marker::Sync,
        {
            type Level = L;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut ::poprako_rdb_core::RdbContext<L>,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                let rows = #obj_module::load_many(context.conn(), oper.ids)
                    .await?;

                #obj_module::decode_many::<#obj>(rows)
            }
        }

        impl<'a, P> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::GenObjUrls<'a, #obj>,
        > for #view<P>
        where
            P: ::poprako_obj_dept::pool::ObjDeptPoolView + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GenObjUrls<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::url::ObjUrls,
                >,
            > {
                //
                ::poprako_obj_dept::pool::gen_urls_bounded(
                    self.pool(),
                    oper.spec,
                    oper.metas,
                )
                .await
            }
        }
    }
}

// Generates total urls.
fn total_urls(obj: &Ident, dept: &Ident) -> TokenStream {
    //
    quote! {
        impl<'a, P, M> ::poprako_orchestra::Run<
            ::poprako_obj_dept::oper::GenObjUrls<'a, #obj>,
        > for #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm + ::core::marker::Sync,
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GenObjUrls<'a, #obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::url::ObjUrls,
                >,
            > {
                //
                ::poprako_obj_dept::pool::gen_urls_bounded(
                    self.pool(),
                    oper.spec,
                    oper.metas,
                )
                .await
            }
        }
    }
}
