use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path};

/// Generates transaction-side single and batch task creation.
pub fn expand_module(table: &Path) -> TokenStream {
    //
    let task_records = task_records(table);

    let validate_identities = validate_identities();

    quote! {
        #task_records

        pub struct PreparedTask {
            pub id: String,

            oper: &'static str,

            key: ObjKeyRecord,

            visible_at: ::time::OffsetDateTime,
        }

        impl PreparedTask {
            //
            pub fn new(
                topic: &str,
                oper: &'static str,
                key: ObjKeyRecord,
                visible_at: ::time::OffsetDateTime,
            ) -> Self {
                //
                let id = ::poprako_obj_dept::model::task::obj_task_id(
                    topic,
                    oper,
                    &key,
                    ORDINARY_GENERATION,
                );

                Self {
                    id,
                    oper,
                    key,
                    visible_at,
                }
            }
        }

        pub async fn defer_tasks(
            conn: &mut RdbConn,
            topic: &str,
            tasks: &[PreparedTask],
        ) -> ObjDeptRest<()> {
            use ::std::collections::HashMap;

            if tasks.is_empty() {
                return Ok(());
            }

            let items = tasks
                .iter()
                .map(|task| TaskItem {
                    id: &task.id,
                    topic,
                    oper: task.oper,
                    obj_id: &task.key.id,
                    ver: i64::from(task.key.ver),
                    domain: &task.key.domain,
                    gen_no: ORDINARY_GENERATION,
                    status: PENDING,
                    visible_at: task.visible_at,
                })
                .collect::<Vec<_>>();

            ::diesel::insert_into(#table::table)
                .values(&items)
                .on_conflict(#table::f_id)
                .do_update()
                .set(#table::f_id.eq(::diesel::upsert::excluded(#table::f_id)))
                .execute(conn)
                .await
                .map_err(diesel_error)?;

            let ids = tasks.iter().map(|task| &task.id).collect::<Vec<_>>();
            let records = #table::table
                .filter(#table::f_id.eq_any(ids))
                .select(TaskIdentityRecord::as_select())
                .load::<TaskIdentityRecord>(conn)
                .await
                .map_err(diesel_error)?;
            #validate_identities

            Ok(())
        }
    }
}

/// Generates the public transaction-side adapter implementation.
pub fn expand_impl(name: &Ident, module: &Ident) -> TokenStream {
    //
    let defer_check = single_check(module);

    //
    quote! {
        impl<L> ::poprako_obj_dept::prom::ObjDeptPromDefer<
            ::poprako_rdb_core::RdbContext<L>,
        > for #name
        where
            L: ::poprako_orchestra::Level + Send,
        {
            #defer_check

            fn defer_delete<
                'a,
                K: ::poprako_obj_dept::key::ObjRegistration,
            >(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                key: &'a ::poprako_obj_dept::key::ObjKey<K>,
            ) -> impl ::std::future::Future<
                Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
            > + Send
            {
                self.defer_deletes(context, ::std::slice::from_ref(key))
            }

            fn defer_checks<
                'a,
                K: ::poprako_obj_dept::key::ObjRegistration,
            >(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                checks: &'a [::poprako_obj_dept::prom::ObjDeptPromCheck<K>],
            ) -> impl ::std::future::Future<
                Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
            > + Send
            {
                let topic = K::TOPIC;

                let tasks = checks
                    .iter()
                    .map(|check| {
                        let visible_at = check
                            .expires_at()
                            .checked_add(::time::Duration::minutes(1))
                            .ok_or_else(|| {
                                ::poprako_obj_dept::rest::ObjDeptError::
                                    Unrecoverable {
                                    message:
                                        "object check visibility \
                                            overflow".into(),
                                }
                            })?;

                        Ok(#module::PreparedTask::new(
                            topic,
                            ::poprako_obj_dept::model::task::CHECK,
                            check.key().to_record(),
                            visible_at,
                        ))
                    })
                    .collect::<::poprako_obj_dept::rest::
                        ObjDeptRest<Vec<_>>>();

                async move {
                    //
                    let mut tasks = tasks?;

                    tasks.sort_unstable_by(|left, right| {
                        left.id.cmp(&right.id)
                    });

                    tasks.dedup_by(|left, right| left.id == right.id);

                    #module::defer_tasks(context.conn(), topic, &tasks).await
                }
            }

            fn defer_deletes<
                'a,
                K: ::poprako_obj_dept::key::ObjRegistration,
            >(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                keys: &'a [::poprako_obj_dept::key::ObjKey<K>],
            ) -> impl ::std::future::Future<
                Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
            > + Send
            {
                let topic = K::TOPIC;

                let now = ::time::OffsetDateTime::now_utc();
                let mut tasks = keys
                    .iter()
                    .map(|key| {
                        #module::PreparedTask::new(
                            topic,
                            ::poprako_obj_dept::model::task::DELETE,
                            key.to_record(),
                            now,
                        )
                    })
                    .collect::<Vec<_>>();

                tasks.sort_unstable_by(|left, right| left.id.cmp(&right.id));
                tasks.dedup_by(|left, right| left.id == right.id);

                async move {
                    #module::defer_tasks(context.conn(), topic, &tasks).await
                }
            }
        }
    }
}

// Generates task records.
fn task_records(table: &Path) -> TokenStream {
    //
    quote! {
        //
        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct TaskIdentityRecord {
            #[diesel(column_name = f_id)]
            id: String,

            #[diesel(column_name = f_topic)]
            topic: String,
            #[diesel(column_name = f_oper)]
            oper: String,

            #[diesel(column_name = f_obj_id)]
            obj_id: String,
            #[diesel(column_name = f_version)]
            ver: i64,
            #[diesel(column_name = f_domain)]
            domain: ::serde_json::Value,

            #[diesel(column_name = f_generation)]
            gen_no: i64,
            #[diesel(column_name = f_status)]
            status: String,
        }

        #[derive(::diesel::Insertable)]
        #[diesel(table_name = #table)]
        struct TaskItem<'a> {
            #[diesel(column_name = f_id)]
            id: &'a str,

            #[diesel(column_name = f_topic)]
            topic: &'a str,
            #[diesel(column_name = f_oper)]
            oper: &'a str,

            #[diesel(column_name = f_obj_id)]
            obj_id: &'a str,
            #[diesel(column_name = f_version)]
            ver: i64,
            #[diesel(column_name = f_domain)]
            domain: &'a ::serde_json::Value,

            #[diesel(column_name = f_generation)]
            gen_no: i64,
            #[diesel(column_name = f_status)]
            status: &'a str,
            #[diesel(column_name = f_visible_at)]
            visible_at: ::time::OffsetDateTime,
        }
    }
}

// Generates validate identities.
fn validate_identities() -> TokenStream {
    //
    quote! {
        //
        let records = records
            .into_iter()
            .map(|record| {
                //
                let TaskIdentityRecord {
                    id,
                    topic,
                    oper,
                    obj_id,
                    ver,
                    domain,
                    gen_no: gen_value,
                    status,
                } = record;

                (
                    id,
                    (
                        topic,
                        oper,
                        obj_id,
                        ver,
                        domain,
                        gen_value,
                        status,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        if records.len() != tasks.len() {
            //
            return Err(ObjDeptError::Unrecoverable {
                message: "object task batch identity is incomplete".into(),
            });
        }

        for task in tasks {
            //
            let Some(record) = records.get(&task.id) else {
                //
                return Err(ObjDeptError::Unrecoverable {
                    message: "object task batch identity is incomplete"
                        .into(),
                });
            };

            let (
                record_topic,
                record_oper,
                record_obj_id,
                record_ver,
                record_domain,
                record_gen,
                record_status,
            ) = record;

            let is_same_identity = record_topic == topic
                && record_oper == task.oper
                && record_obj_id == &task.key.id
                && *record_ver == i64::from(task.key.ver)
                && record_domain == &task.key.domain
                && *record_gen == ORDINARY_GENERATION;

            if !is_same_identity {
                //
                return Err(ObjDeptError::Unrecoverable {
                    message: "object task identity conflict".into(),
                });
            }

            match record_status.as_str() {
                //
                PENDING | PROCESSING => {}

                OPERATOR => {
                    //
                    return
                        Err(ObjDeptError::
                            conflict("object task requires operator \
                                repair".into()));
                }

                _ => {
                    //
                    return Err(ObjDeptError::Unrecoverable {
                        message: "invalid object task status".into(),
                    });
                }
            }
        }
    }
}

// Generates the single Check without cloning a typed key.
fn single_check(module: &Ident) -> TokenStream {
    //
    quote! {
            fn defer_check<
                'a,
                K: ::poprako_obj_dept::key::ObjRegistration,
            >(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                key: &'a ::poprako_obj_dept::key::ObjKey<K>,
                expires_at: ::time::OffsetDateTime,
            ) -> impl ::std::future::Future<
                Output = ::poprako_obj_dept::rest::ObjDeptRest<()>,
            > + Send
            {
                //
                let topic = K::TOPIC;

                let key = key.to_record();

                async move {
                    //
                    let visible_at = expires_at
                        .checked_add(::time::Duration::minutes(1))
                        .ok_or_else(|| {
                            ::poprako_obj_dept::rest::ObjDeptError::
                                Unrecoverable {
                                message: "object check visibility overflow"
                                    .into(),
                            }
                        })?;

                    let task = #module::PreparedTask::new(
                        topic,
                        ::poprako_obj_dept::model::task::CHECK,
                        key,
                        visible_at,
                    );

                    #module::defer_tasks(context.conn(), topic, &[task]).await
                }
            }

    }
}
