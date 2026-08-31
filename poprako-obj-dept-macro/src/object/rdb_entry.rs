use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

/// Generates typed Diesel storage for one latest-object table.
#[expect(
    clippy::too_many_lines,
    reason = "one generated typed RDB-entry implementation is audited as a unit"
)]
pub fn expand(table: &Path) -> TokenStream {
    //
    quote! {
        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct FullRow {
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
        struct ObjStateRow {
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
        struct ObjPresenceStateRow {
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

        impl From<ObjStateRow> for ::poprako_obj_dept::rdb_impl::ObjRdbRow {
            //
            fn from(row: ObjStateRow) -> Self {
                //
                Self {
                    ver: row.ver,
                    key: row.key,
                    f_is_uploaded: row.f_is_uploaded,
                    hash: row.hash,
                    ext: row.ext,
                }
            }
        }

        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct ObjStateEntryRow {
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
            pub row: ::poprako_obj_dept::rdb_impl::ObjRdbRow,
        }

        impl From<ObjStateEntryRow> for ObjRdbEntry {
            //
            fn from(row: ObjStateEntryRow) -> Self {
                //
                Self {
                    id: row.id,
                    row: ::poprako_obj_dept::rdb_impl::ObjRdbRow {
                        ver: row.ver,
                        key: row.key,
                        f_is_uploaded: row.f_is_uploaded,
                        hash: row.hash,
                        ext: row.ext,
                    },
                }
            }
        }

        pub fn decode_many<K>(
            rows: Vec<ObjRdbEntry>,
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

            for row_entry in rows {
                //
                let id = row_entry.id;

                let Some(obj_meta) =
                    ::poprako_obj_dept::rdb_impl::decode_row::<K>(
                        &id,
                        row_entry.row,
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
        struct ObjAnchorRow<'a> {
            #[diesel(column_name = f_id)]
            id: &'a str,
            #[diesel(column_name = f_version)]
            ver: i64,
        }

        #[derive(::diesel::Insertable)]
        #[diesel(table_name = #table)]
        struct ObjWriteRow<'a> {
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

        pub async fn load(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
            lock: bool,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<
            Option<::poprako_obj_dept::rdb_impl::ObjRdbRow>,
        > {
            use ::diesel::OptionalExtension as _;
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            let row = match lock {
                true => #table::table
                    .filter(#table::f_id.eq(id))
                    .for_update()
                    .select(ObjStateRow::as_select())
                    .first::<ObjStateRow>(conn)
                    .await,
                false => #table::table
                    .filter(#table::f_id.eq(id))
                    .select(ObjStateRow::as_select())
                    .first::<ObjStateRow>(conn)
                    .await,
            }
            .optional()
            .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

            Ok(row.map(Into::into))
        }

        pub async fn load_many(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[String],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            load_many_inner(conn, ids, false).await
        }

        pub async fn load_for_presence_reconciliation(
            conn: &mut ::poprako_rdb_core::RdbConn,
            id: &str,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<
            Option<(
                ::poprako_obj_dept::rdb_impl::ObjRdbRow,
                ::time::OffsetDateTime,
            )>,
        > {
            use ::diesel::OptionalExtension as _;
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            let row = #table::table
                .filter(#table::f_id.eq(id))
                .select(ObjPresenceStateRow::as_select())
                .first::<ObjPresenceStateRow>(conn)
                .await
                .optional()
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

            Ok(row.map(|row| {
                let state = ::poprako_obj_dept::rdb_impl::ObjRdbRow {
                    ver: row.ver,
                    key: row.key,
                    f_is_uploaded: row.f_is_uploaded,
                    hash: row.hash,
                    ext: row.ext,
                };

                (state, row.revision)
            }))
        }

        pub async fn load_many_for_update(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[String],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            load_many_inner(conn, ids, true).await
        }

        async fn load_many_inner(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[String],
            lock: bool,
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let rows = match lock {
                true => #table::table
                    .filter(#table::f_id.eq_any(ids))
                    .order(#table::f_id.asc())
                    .for_update()
                    .select(ObjStateEntryRow::as_select())
                    .load::<ObjStateEntryRow>(conn)
                    .await,
                false => #table::table
                    .filter(#table::f_id.eq_any(ids))
                    .order(#table::f_id.asc())
                    .select(ObjStateEntryRow::as_select())
                    .load::<ObjStateEntryRow>(conn)
                    .await,
            }
            .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

            Ok(rows.into_iter().map(Into::into).collect())
        }

        pub async fn ensure_anchors(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[String],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<Vec<ObjRdbEntry>> {
            use ::diesel::prelude::ExpressionMethods as _;
            use ::diesel::SelectableHelper as _;
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let anchors = ids
                .iter()
                .map(|id| ObjAnchorRow {
                    id,
                    ver: 0,
                })
                .collect::<Vec<_>>();

            ::diesel::insert_into(#table::table)
                .values(&anchors)
                .on_conflict(#table::f_id)
                .do_update()
                .set(#table::f_version.eq(#table::f_version))
                .returning(ObjStateEntryRow::as_returning())
                .get_results::<ObjStateEntryRow>(conn)
                .await
                .map(|rows| rows.into_iter().map(Into::into).collect())
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)
        }

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

            let rows = writes
                .iter()
                .map(|write| ObjWriteRow {
                    id: write.id,
                    ver: i64::from(write.ver),
                    key: write.key,
                    f_is_uploaded: false,
                    hash: write.hash,
                    ext: write.ext,
                })
                .collect::<Vec<_>>();

            ::diesel::insert_into(#table::table)
                .values(&rows)
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
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

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
            ids: &[String],
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
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

            Ok(())
        }

        pub async fn remove_many(
            conn: &mut ::poprako_rdb_core::RdbConn,
            ids: &[String],
        ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel_async::RunQueryDsl as _;

            if ids.is_empty() {
                return Ok(());
            }

            ::diesel::delete(#table::table.filter(#table::f_id.eq_any(ids)))
                .execute(conn)
                .await
                .map_err(::poprako_obj_dept::rdb_impl::diesel_err)?;

            Ok(())
        }

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
            .map_err(::poprako_obj_dept::rdb_impl::diesel_err)
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
            .map_err(::poprako_obj_dept::rdb_impl::diesel_err)
        }

        #[allow(dead_code)]
        fn assert_full_schema(row: FullRow) {
            drop(row);
        }
    }
}
