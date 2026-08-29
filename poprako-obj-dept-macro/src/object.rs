use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Path, Result, Token, braced};

// One object declaration in the total manifest.
struct ObjInput {
    // Compile-time object marker.
    marker: Ident,
    // Diesel table path.
    table: Path,
    // Durable task topic.
    topic: LitStr,
    // Physical storage namespace.
    namespace: LitStr,
}

impl Parse for ObjInput {
    // Parses one object declaration.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let marker = input.parse()?;

        let content;

        braced!(content in input);

        parse_field(&content, "table")?;

        let table = content.parse()?;

        content.parse::<Token![,]>()?;

        parse_field(&content, "topic")?;

        let topic = content.parse()?;

        content.parse::<Token![,]>()?;

        parse_field(&content, "namespace")?;

        let namespace = content.parse()?;

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        if !content.is_empty() {
            return Err(input.error("unexpected object declaration tokens"));
        }

        Ok(Self {
            marker,
            table,
            topic,
            namespace,
        })
    }
}

// Complete object manifest.
struct ObjsInput(Vec<ObjInput>);

impl Parse for ObjsInput {
    // Parses the complete object manifest.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let mut objs = Vec::new();

        while !input.is_empty() {
            //
            objs.push(input.parse()?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if objs.is_empty() {
            return Err(input.error("expected at least one object"));
        }

        Ok(Self(objs))
    }
}

/// Expands the complete object manifest and typed Diesel operations.
pub fn expand(input: TokenStream) -> Result<TokenStream> {
    //
    let ObjsInput(objs) = syn::parse2(input)?;

    validate_unique(&objs)?;

    let mut modules = Vec::with_capacity(objs.len());

    for obj in &objs {
        modules.push(expand_obj(obj));
    }

    let unique_markers = objs.iter().map(|obj| &obj.marker);

    let unique_tables = objs.iter().map(|obj| &obj.table);

    let manifest = objs.iter().map(|obj| {
        //
        let marker = &obj.marker;

        let module =
            format_ident!("__obj_dept_{}", to_snake_case(&marker.to_string()),);

        let topic = &obj.topic;

        let namespace = &obj.namespace;

        quote!((#marker, #module, #topic, #namespace),)
    });

    Ok(quote! {
        #[doc(hidden)]
        mod __obj_dept_unique {
            trait Marker {}

            #(impl Marker for super::#unique_markers {})*

            trait Table {}

            #(impl Table for super::#unique_tables::table {})*
        }

        #(#modules)*

        macro_rules! __objs_manifest {
            ($callback:ident) => {
                $callback! {
                    #(#manifest)*
                }
            };
        }
    })
}

// Rejects duplicate marker, table, topic, and namespace values.
fn validate_unique(objs: &[ObjInput]) -> Result<()> {
    //
    let mut markers = HashSet::new();

    let mut tables = HashSet::new();

    let mut topics = HashSet::new();

    let mut namespaces = HashSet::new();

    for obj in objs {
        //
        validate_value(
            &mut markers,
            obj.marker.to_string(),
            &obj.marker,
            "marker",
        )?;

        let table = &obj.table;

        validate_value(
            &mut tables,
            quote!(#table).to_string(),
            &obj.marker,
            "table",
        )?;

        validate_value(&mut topics, obj.topic.value(), &obj.marker, "topic")?;

        validate_value(
            &mut namespaces,
            obj.namespace.value(),
            &obj.marker,
            "namespace",
        )?;
    }

    Ok(())
}

// Expands typed Diesel helpers for one object declaration.
fn expand_obj(obj: &ObjInput) -> TokenStream {
    //
    let marker = &obj.marker;

    let table = &obj.table;

    let topic = &obj.topic;

    let namespace = &obj.namespace;

    let module =
        format_ident!("__obj_dept_{}", to_snake_case(&marker.to_string()),);

    let load = expand_load(table);

    let write = expand_write(table);

    let detach = expand_detach(table);

    let verify = expand_verify(table);

    let retire = expand_retire(table);

    let remove = expand_remove(table);

    quote! {
        #[doc(hidden)]
        mod #module {
            use super::#table;

            pub const TOPIC: &str = #topic;
            pub const NAMESPACE: &str = #namespace;

            #load
            #write
            #detach
            #verify
            #retire
            #remove
        }
    }
}

// Inserts one manifest value or reports its duplicate.
fn validate_value(
    values: &mut HashSet<String>,
    value: String,
    marker: &Ident,
    kind: &str,
) -> Result<()> {
    //
    if values.insert(value) {
        return Ok(());
    }

    Err(syn::Error::new(
        marker.span(),
        format!("duplicate object {}", kind),
    ))
}

// Generates the standardized row-load implementation.
fn expand_load(table: &Path) -> TokenStream {
    //
    quote! {
        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct FullRow {
            #[diesel(column_name = f_id)]
            id: String,
            #[diesel(column_name = f_version)]
            version: i64,
            #[diesel(column_name = f_is_uploaded)]
            f_is_uploaded: Option<bool>,
            #[diesel(column_name = f_hash)]
            hash: Option<Vec<u8>>,
            #[diesel(column_name = f_ext)]
            ext: Option<String>,
            #[diesel(column_name = f_created_at)]
            created_at: ::time::OffsetDateTime,
            #[diesel(column_name = f_updated_at)]
            updated_at: ::time::OffsetDateTime,
        }

        impl From<FullRow> for ::poprako_obj_dept::rdb_impl::ObjRdbRow {
            //
            fn from(row: FullRow) -> Self {
                //
                let FullRow {
                    //
                    id,
                    version,
                    f_is_uploaded,
                    hash,
                    ext,
                    created_at,
                    updated_at,
                } = row;

                drop((id, created_at, updated_at));

                Self {
                    version,
                    f_is_uploaded,
                    hash,
                    ext,
                }
            }
        }

        pub fn load<'a>(
            conn: &'a mut ::poprako_rdb_core::RdbConn,
            id: &'a str,
            lock: bool,
        ) -> impl ::std::future::Future<
            Output = ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::rdb_impl::ObjRdbRow>,
            >,
        > + Send {
            async move {
                use ::diesel::OptionalExtension as _;
                use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
                use ::diesel::SelectableHelper as _;
                use ::diesel_async::RunQueryDsl as _;

                let row = match lock {
                    true => #table::table
                        .filter(#table::f_id.eq(id))
                        .for_update()
                        .select(FullRow::as_select())
                        .first::<FullRow>(conn)
                        .await,

                    false => #table::table
                        .filter(#table::f_id.eq(id))
                        .select(FullRow::as_select())
                        .first::<FullRow>(conn)
                        .await,
                }
                .optional()
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

                Ok(row.map(Into::into))
            }
        }
    }
}

// Generates the standardized row-write implementation.
fn expand_write(table: &Path) -> TokenStream {
    //
    quote! {
        pub fn write<'a>(
            conn: &'a mut ::poprako_rdb_core::RdbConn,
            write: ::poprako_obj_dept::rdb_impl::ObjRdbWrite<'a>,
        ) -> impl ::std::future::Future<
            Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
        > + Send {
            async move {
                use ::diesel::prelude::ExpressionMethods as _;
                use ::diesel_async::RunQueryDsl as _;

                ::diesel::insert_into(#table::table)
                    .values((
                        #table::f_id.eq(write.id),
                        #table::f_version.eq(i64::from(write.version)),
                        #table::f_is_uploaded.eq(false),
                        #table::f_hash.eq(write.hash),
                        #table::f_ext.eq(write.ext),
                    ))
                    .on_conflict(#table::f_id)
                    .do_update()
                    .set((
                        #table::f_version.eq(i64::from(write.version)),
                        #table::f_is_uploaded.eq(false),
                        #table::f_hash.eq(write.hash),
                        #table::f_ext.eq(write.ext),
                        #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                    ))
                    .execute(conn)
                    .await
                    .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

                Ok(())
            }
        }
    }
}

// Generates the standardized object-detach implementation.
fn expand_detach(table: &Path) -> TokenStream {
    //
    quote! {
        pub fn detach<'a>(
            conn: &'a mut ::poprako_rdb_core::RdbConn,
            id: &'a str,
        ) -> impl ::std::future::Future<
            Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
        > + Send {
            async move {
                use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
                use ::diesel_async::RunQueryDsl as _;

                ::diesel::update(#table::table.filter(#table::f_id.eq(id)))
                    .set((
                        #table::f_is_uploaded.eq(None::<bool>),
                        #table::f_hash.eq(None::<Vec<u8>>),
                        #table::f_ext.eq(None::<String>),
                        #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                    ))
                    .execute(conn)
                    .await
                    .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

                Ok(())
            }
        }
    }
}

// Generates the standardized upload-verification implementation.
fn expand_verify(table: &Path) -> TokenStream {
    //
    quote! {
        pub fn verify(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            version: u32,
        ) -> impl ::std::future::Future<
            Output = ::poprako_obj_dept::rest::ObjDeptRest<usize>,
        > + Send {
            async move {
                use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
                use ::diesel_async::RunQueryDsl as _;

                let updated = ::diesel::update(
                    #table::table
                        .filter(#table::f_id.eq(id))
                        .filter(#table::f_version.eq(i64::from(version)))
                        .filter(#table::f_is_uploaded.eq(false))
                        .filter(#table::f_hash.is_not_null())
                        .filter(#table::f_ext.is_not_null()),
                )
                .set((
                    #table::f_is_uploaded.eq(true),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

                Ok(updated)
            }
        }
    }
}

// Generates the standardized object-retirement implementation.
fn expand_retire(table: &Path) -> TokenStream {
    //
    quote! {
        pub fn retire(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            version: u32,
        ) -> impl ::std::future::Future<
            Output = ::poprako_obj_dept::rest::ObjDeptRest<usize>,
        > + Send {
            async move {
                use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
                use ::diesel_async::RunQueryDsl as _;

                let updated = ::diesel::update(
                    #table::table
                        .filter(#table::f_id.eq(id))
                        .filter(#table::f_version.eq(i64::from(version)))
                        .filter(#table::f_is_uploaded.eq(false))
                        .filter(#table::f_hash.is_not_null())
                        .filter(#table::f_ext.is_not_null()),
                )
                .set((
                    #table::f_is_uploaded.eq(None::<bool>),
                    #table::f_hash.eq(None::<Vec<u8>>),
                    #table::f_ext.eq(None::<String>),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

                Ok(updated)
            }
        }
    }
}

// Generates the standardized object-removal implementation.
fn expand_remove(table: &Path) -> TokenStream {
    //
    quote! {
        pub fn remove<'a>(
            conn: &'a mut ::poprako_rdb_core::RdbConn,
            id: &'a str,
        ) -> impl ::std::future::Future<
            Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
        > + Send {
            async move {
                use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
                use ::diesel_async::RunQueryDsl as _;

                ::diesel::delete(#table::table.filter(#table::f_id.eq(id)))
                    .execute(conn)
                    .await
                    .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

                Ok(())
            }
        }
    }
}

// Converts a Rust type identifier into its snake-case namespace component.
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

// Parses one exact named field.
fn parse_field(input: ParseStream<'_>, expected: &str) -> Result<()> {
    //
    let field = input.parse::<Ident>()?;

    if field != expected {
        //
        return Err(syn::Error::new(
            field.span(),
            format!("expected `{}`", expected),
        ));
    }

    input.parse::<Token![:]>()?;

    Ok(())
}
