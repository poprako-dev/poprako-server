//! In-memory ObjDept operations used by server tests.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use time::{Duration, OffsetDateTime};
use url::Url;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::ObjSlot;
use poprako_obj_dept::model::task::ObjTask;
use poprako_obj_dept::oper::{DelObjs, GenObjSlot};
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};

pub fn gen_url(
    namespace: &str,
    meta: Option<&ObjMeta>,
) -> ObjDeptRest<Option<Url>> {
    let Some(meta) = meta else {
        return Ok(None);
    };

    if !meta.f_is_uploaded {
        return Ok(None);
    }

    let key = meta.key.encode(namespace);

    Url::parse(&format!("https://obj.test/{}", key))
        .map(Some)
        .map_err(|source| ObjDeptError::Unrecoverable {
            message: source.to_string(),
        })
}

pub fn gen_slot(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    namespace: &str,
    oper: &GenObjSlot<'_, impl Sized>,
) -> ObjDeptRest<ObjSlot> {
    let prev = objs.get(oper.spec.id).cloned();

    let version = prev.as_ref().map_or(Ok(1), |prev| {
        prev.version
            .checked_add(1)
            .ok_or_else(|| ObjDeptError::Unrecoverable {
                message: "object version overflow".into(),
            })
    })?;

    let key = ObjKey {
        id: oper.spec.id.to_owned(),
        version,
    };

    if let Some(prev_key) = prev.and_then(|prev| prev.meta.map(|meta| meta.key))
    {
        tasks.push((topic, ObjTask::Delete { key: prev_key }));
    }

    let meta = ObjMeta {
        key: key.clone(),
        f_is_uploaded: false,
        hash: oper.spec.hash.to_vec(),
        ext: oper.spec.ext.to_owned(),
    };

    objs.insert(
        oper.spec.id.to_owned(),
        MockObjRecord {
            version,
            meta: Some(meta),
        },
    );

    tasks.push((topic, ObjTask::Check { key: key.clone() }));

    let physical_key = key.encode(namespace);

    let url = Url::parse(&format!("https://obj.test/write/{}", physical_key))
        .map_err(|source| ObjDeptError::Unrecoverable {
        message: source.to_string(),
    })?;

    Ok(ObjSlot {
        key,
        url,
        headers: Default::default(),
        expires_at: OffsetDateTime::now_utc() + Duration::minutes(5),
    })
}

pub fn del_objs<B>(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    oper: &DelObjs<'_, B>,
) {
    let ids = match oper {
        DelObjs::Detach { ids, .. } | DelObjs::Remove { ids, .. } => ids,
    };

    for id in *ids {
        let Some(record) = objs.get_mut(id) else {
            continue;
        };

        if let Some(meta) = record.meta.take() {
            tasks.push((topic, ObjTask::Delete { key: meta.key }));
        }
    }

    if matches!(oper, DelObjs::Remove { .. }) {
        for id in *ids {
            objs.remove(id);
        }
    }
}

#[macro_export]
macro_rules! __impl_mock_obj_dept {
    ($obj:ty, $topic:literal, $namespace:literal) => {
        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::GetObjMeta<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GetObjMeta<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::model::meta::ObjMeta>,
            > {
                let state = self.state.lock().unwrap();

                Ok(state
                    .objs
                    .get($topic)
                    .and_then(|objs| objs.get(oper.id))
                    .and_then(|record| record.meta.clone()))
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::GetObjMeta<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::GetObjMeta<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::model::meta::ObjMeta>,
            > {
                Ok(context
                    .state
                    .objs
                    .get($topic)
                    .and_then(|objs| objs.get(oper.id))
                    .and_then(|record| record.meta.clone()))
            }
        }

        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::GenObjUrl<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GenObjUrl<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<Option<::url::Url>> {
                let state = self.state.lock().unwrap();

                let meta = state
                    .objs
                    .get($topic)
                    .and_then(|objs| objs.get(oper.id))
                    .and_then(|record| record.meta.as_ref());

                $crate::part_impl::obj_dept::mock_impl::gen_url(
                    $namespace, meta,
                )
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::GenObjSlot<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::GenObjSlot<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::poprako_obj_dept::model::slot::ObjSlot,
            > {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::gen_slot(
                    objs, obj_tasks, $topic, $namespace, oper,
                )
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::DelObjs<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::DelObjs<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::del_objs(
                    objs, obj_tasks, $topic, oper,
                );

                Ok(())
            }
        }
    };
}
