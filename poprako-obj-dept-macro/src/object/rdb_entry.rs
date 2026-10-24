use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

/// Generates typed Diesel storage for one latest-object table.
pub fn expand(table: &Path) -> TokenStream {
    //
    let write_and_cleanup_rows = write_and_cleanup_rows(table);

    let lock_and_anchor_rows = lock_and_anchor_rows(table);

    let load_rows = load_rows(table);

    let decode_entries = decode_entries(table);

    let upload_marks = upload_marks(table);

    quote! {
        //
        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct FullRecord {
            #[diesel(column_name = f_id)]
            id: String,

            #[diesel(column_name = f_version)]
            ver: i64,

            #[diesel(column_name = f_key)]
            key: Option<String>,
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

        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct ObjStateRecord {
            #[diesel(column_name = f_version)]
            ver: i64,

            #[diesel(column_name = f_key)]
            key: Option<String>,
            #[diesel(column_name = f_is_uploaded)]
            f_is_uploaded: Option<bool>,
            #[diesel(column_name = f_hash)]
            hash: Option<Vec<u8>>,
            #[diesel(column_name = f_ext)]
            ext: Option<String>,
        }

        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct ObjPresenceStateRecord {
            #[diesel(column_name = f_version)]
            ver: i64,

            #[diesel(column_name = f_key)]
            key: Option<String>,
            #[diesel(column_name = f_is_uploaded)]
            f_is_uploaded: Option<bool>,
            #[diesel(column_name = f_hash)]
            hash: Option<Vec<u8>>,
            #[diesel(column_name = f_ext)]
            ext: Option<String>,

            #[diesel(column_name = f_updated_at)]
            revision: ::time::OffsetDateTime,
        }

        impl From<ObjStateRecord>
            for ::poprako_obj_dept::rdb_impl::ObjRdbRecord
        {
            //
            fn from(record: ObjStateRecord) -> Self {
                Self {
                    ver: record.ver,
                    key: record.key,
                    f_is_uploaded: record.f_is_uploaded,
                    hash: record.hash,
                    ext: record.ext,
                }
            }
        }

        #decode_entries

        #load_rows

        #lock_and_anchor_rows

        #write_and_cleanup_rows

        #upload_marks

        const _: fn(FullRecord) = |record| {
            //
            let FullRecord {
                id, ver, key, f_is_uploaded, hash, ext, created_at, updated_at,
            } = record;

            drop((
                id, ver, key, f_is_uploaded, hash, ext, created_at, updated_at,
            ));
        };
    }
}

// Generates write and cleanup rows.
fn write_and_cleanup_rows(table: &Path) -> TokenStream {
    //
    quote! {
        pub async fn write_many(
            conn: &mut ::poprako_rdb_core::RdbConn,
            writes: &[::poprako_obj_dept::rdb_impl::ObjRdbWrite<'_>],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
            use ::diesel::prelude::ExpressionMethods as _;
            use ::diesel::upsert::excluded;
            use ::diesel_async::RunQueryDsl as _;

            if writes.is_empty() {
                return Ok(());
            }

            let items = writes
                .iter()
                .map(|write| ObjWriteItem {
                    id: write.id,
                    ver: i64::from(write.ver),
                    key: write.key,
                    f_is_uploaded: false,
                    hash: write.hash,
                    ext: write.ext,
                })
                .collect::<Vec<_>>();

            ::diesel::insert_into(#table::table)
                .values(&items)
                .on_conflict(#table::f_id)
                .do_update()
                .set((
                    #table::f_version.eq(excluded(#table::f_version)),
                    #table::f_key.eq(excluded(#table::f_key)),
                    #table::f_is_uploaded.eq(false),
                    #table::f_hash.eq(excluded(#table::f_hash)),
                    #table::f_ext.eq(excluded(#table::f_ext)),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(::poprako_obj_dept::rdb_impl::diesel_error)?;

            Ok(())
        }

        pub async fn write(
            conn: &mut ::poprako_rdb_core::RdbConn,
            write: ::poprako_obj_dept::rdb_impl::ObjRdbWrite<'_>,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
            write_many(conn, &[write]).await
        }

        pub async fn detach_many(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[&str],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(());
            }

            ::diesel::update(#table::table.filter(#table::f_id.eq_any(ids)))
                .set((
                    #table::f_is_uploaded.eq(None::<bool>),
                    #table::f_key.eq(None::<String>),
                    #table::f_hash.eq(None::<Vec<u8>>),
                    #table::f_ext.eq(None::<String>),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(::poprako_obj_dept::rdb_impl::diesel_error)?;

            Ok(())
        }

        pub async fn remove_many(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[&str],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(());
            }

            ::diesel::delete(#table::table.filter(#table::f_id.eq_any(ids)))
                .execute(conn)
                .await
                .map_err(::poprako_obj_dept::rdb_impl::diesel_error)?;

            Ok(())
        }
    }
}

// Generates lock and anchor rows.
fn lock_and_anchor_rows(table: &Path) -> TokenStream {
    //
    quote! {
        pub async fn load_many_for_update(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[&str],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            load_many_inner(conn, ids, true).await
        }

        async fn load_many_inner(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[&str],
            lock: bool,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let records = match lock {
                true => #table::table
                    .filter(#table::f_id.eq_any(ids))
                    .order(#table::f_id.asc())
                    .for_update()
                    .select(ObjStateEntryRecord::as_select())
                    .load::<ObjStateEntryRecord>(conn)
                    .await,
                false => #table::table
                    .filter(#table::f_id.eq_any(ids))
                    .order(#table::f_id.asc())
                    .select(ObjStateEntryRecord::as_select())
                    .load::<ObjStateEntryRecord>(conn)
                    .await,
            }
            .map_err(::poprako_obj_dept::rdb_impl::diesel_error)?;

            Ok(records.into_iter().map(Into::into).collect())
        }

        pub async fn ensure_anchors(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[&str],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            use ::diesel::prelude::ExpressionMethods as _;
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let anchors = ids
                .iter()
                .map(|id| ObjAnchorItem {
                    id,
                    ver: 0,
                })
                .collect::<Vec<_>>();

            ::diesel::insert_into(#table::table)
                .values(&anchors)
                .on_conflict(#table::f_id)
                .do_update()
                .set(#table::f_version.eq(#table::f_version))
                .returning(ObjStateEntryRecord::as_returning())
                .get_results::<ObjStateEntryRecord>(conn)
                .await
                .map(|records| records.into_iter().map(Into::into).collect())
                .map_err(::poprako_obj_dept::rdb_impl::diesel_error)
        }
    }
}

// Generates load rows.
fn load_rows(table: &Path) -> TokenStream {
    //
    quote! {
        pub async fn load(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            lock: bool,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<
            Option<::poprako_obj_dept::rdb_impl::ObjRdbRecord>,
        > {
            use ::diesel::OptionalExtension as _;
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            let record = match lock {
                true => #table::table
                    .filter(#table::f_id.eq(id))
                    .for_update()
                    .select(ObjStateRecord::as_select())
                    .first::<ObjStateRecord>(conn)
                    .await,
                false => #table::table
                    .filter(#table::f_id.eq(id))
                    .select(ObjStateRecord::as_select())
                    .first::<ObjStateRecord>(conn)
                    .await,
            }
            .optional()
            .map_err(::poprako_obj_dept::rdb_impl::diesel_error)?;

            Ok(record.map(Into::into))
        }

        pub async fn load_many(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[&str],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            load_many_inner(conn, ids, false).await
        }

        pub async fn load_for_presence_reconciliation(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<
            Option<(
                ::poprako_obj_dept::rdb_impl::ObjRdbRecord,
                ::time::OffsetDateTime,
            )>,
        > {
            use ::diesel::OptionalExtension as _;
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            let record = #table::table
                .filter(#table::f_id.eq(id))
                .select(ObjPresenceStateRecord::as_select())
                .first::<ObjPresenceStateRecord>(conn)
                .await
                .optional()
                .map_err(::poprako_obj_dept::rdb_impl::diesel_error)?;

            Ok(record.map(|record| {
                let state = ::poprako_obj_dept::rdb_impl::ObjRdbRecord {
                    ver: record.ver,
                    key: record.key,
                    f_is_uploaded: record.f_is_uploaded,
                    hash: record.hash,
                    ext: record.ext,
                };

                (state, record.revision)
            }))
        }
    }
}

// Generates decode entries.
fn decode_entries(table: &Path) -> TokenStream {
    //
    quote! {
        //
        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct ObjStateEntryRecord {
            #[diesel(column_name = f_id)]
            id: String,

            #[diesel(column_name = f_version)]
            ver: i64,
            #[diesel(column_name = f_key)]
            key: Option<String>,
            #[diesel(column_name = f_is_uploaded)]
            f_is_uploaded: Option<bool>,
            #[diesel(column_name = f_hash)]
            hash: Option<Vec<u8>>,
            #[diesel(column_name = f_ext)]
            ext: Option<String>,
        }

        pub struct ObjRdbEntry {
            pub id: String,
            pub record: ::poprako_obj_dept::rdb_impl::ObjRdbRecord,
        }

        impl From<ObjStateEntryRecord> for ObjRdbEntry {
            //
            fn from(record: ObjStateEntryRecord) -> Self {
                Self {
                    id: record.id,
                    record: ::poprako_obj_dept::rdb_impl::ObjRdbRecord {
                        ver: record.ver,
                        key: record.key,
                        f_is_uploaded: record.f_is_uploaded,
                        hash: record.hash,
                        ext: record.ext,
                    },
                }
            }
        }

        pub fn decode_many<K>(
            records: Vec<ObjRdbEntry>,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<
            ::std::collections::HashMap<
                String,
                ::poprako_obj_dept::model::meta::ObjMeta,
            >,
        >
        where
            K: ::poprako_obj_dept::key::KeyMap<Img = String>,
        {
            //
            let mut obj_metas = ::std::collections::HashMap::new();

            for entry in records {
                //
                let id = entry.id;

                let Some(obj_meta) =
                    ::poprako_obj_dept::rdb_impl::decode_record::<K>(
                        &id,
                        entry.record,
                    )?
                else {
                    continue;
                };

                obj_metas.insert(id, obj_meta);
            }

            Ok(obj_metas)
        }

        #[derive(::diesel::Insertable)]
        #[diesel(table_name = #table)]
        struct ObjAnchorItem<'a> {
            #[diesel(column_name = f_id)]
            id: &'a str,
            #[diesel(column_name = f_version)]
            ver: i64,
        }

        #[derive(::diesel::Insertable)]
        #[diesel(table_name = #table)]
        struct ObjWriteItem<'a> {
            #[diesel(column_name = f_id)]
            id: &'a str,

            #[diesel(column_name = f_version)]
            ver: i64,

            #[diesel(column_name = f_key)]
            key: &'a str,
            #[diesel(column_name = f_is_uploaded)]
            f_is_uploaded: bool,
            #[diesel(column_name = f_hash)]
            hash: &'a [u8],
            #[diesel(column_name = f_ext)]
            ext: &'a str,
        }
    }
}

// Generates upload marks.
fn upload_marks(table: &Path) -> TokenStream {
    //
    quote! {
        pub async fn mark_uploaded(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            ver: u32,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
            set_uploaded(conn, id, ver, true).await
        }

        pub async fn mark_uploaded_if_revision(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            ver: u32,
            revision: ::time::OffsetDateTime,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
            set_uploaded_if_revision(conn, id, ver, true, revision).await
        }

        pub async fn mark_unuploaded_if_revision(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            ver: u32,
            revision: ::time::OffsetDateTime,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
            set_uploaded_if_revision(conn, id, ver, false, revision).await
        }

        async fn set_uploaded(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            ver: u32,
            f_is_uploaded: bool,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel_async::RunQueryDsl as _;

            ::diesel::update(
                #table::table
                    .filter(#table::f_id.eq(id))
                    .filter(#table::f_version.eq(i64::from(ver)))
                    .filter(#table::f_key.is_not_null())
                    .filter(#table::f_is_uploaded.is_not_null())
                    .filter(#table::f_hash.is_not_null())
                    .filter(#table::f_ext.is_not_null()),
            )
            .set((
                #table::f_is_uploaded.eq(f_is_uploaded),
                #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
            ))
            .execute(conn)
            .await
            .map_err(::poprako_obj_dept::rdb_impl::diesel_error)
        }

        async fn set_uploaded_if_revision(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            ver: u32,
            f_is_uploaded: bool,
            revision: ::time::OffsetDateTime,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel_async::RunQueryDsl as _;

            ::diesel::update(
                #table::table
                    .filter(#table::f_id.eq(id))
                    .filter(#table::f_version.eq(i64::from(ver)))
                    .filter(#table::f_updated_at.eq(revision))
                    .filter(#table::f_key.is_not_null())
                    .filter(#table::f_is_uploaded.is_not_null())
                    .filter(#table::f_hash.is_not_null())
                    .filter(#table::f_ext.is_not_null()),
            )
            .set((
                #table::f_is_uploaded.eq(f_is_uploaded),
                #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
            ))
            .execute(conn)
            .await
            .map_err(::poprako_obj_dept::rdb_impl::diesel_error)
        }
    }
}
