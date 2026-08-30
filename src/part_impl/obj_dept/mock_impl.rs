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
use poprako_obj_dept::model::url::ObjUrls;
use poprako_obj_dept::oper::{GenObjSlot, GenObjSlots, RetireObjs};
use poprako_obj_dept::pool::ObjUrlProfile;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};

pub fn gen_urls(
    namespace: &str,
    meta: Option<&ObjMeta>,
    profile: ObjUrlProfile,
    thumbnail_enabled: bool,
) -> ObjDeptRest<Option<ObjUrls>> {
    let Some(meta) = meta else {
        return Ok(None);
    };

    if !meta.is_available {
        return Ok(None);
    }

    let key = meta.key.encode(namespace);

    let origin_url =
        Url::parse(&format!("https://obj.test/{}", key)).map_err(|source| {
            ObjDeptError::Unrecoverable {
                message: source.to_string(),
            }
        })?;
    let thumbnail_url = match (profile, thumbnail_enabled) {
        (ObjUrlProfile::ImageThumbnail, true) => Some(
            Url::parse(&format!("https://obj.test/thumbnail/{}", key))
                .map_err(|source| ObjDeptError::Unrecoverable {
                    message: source.to_string(),
                })?,
        ),
        (ObjUrlProfile::OriginOnly | ObjUrlProfile::ImageThumbnail, false)
        | (ObjUrlProfile::OriginOnly, true) => None,
    };

    Ok(Some(ObjUrls {
        origin_url,
        thumbnail_url,
    }))
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
        is_available: false,
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

pub fn gen_slots(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    namespace: &str,
    oper: &GenObjSlots<'_, impl Sized>,
) -> ObjDeptRest<HashMap<String, ObjSlot>> {
    let mut ids = oper.specs.iter().map(|spec| spec.id).collect::<Vec<_>>();

    ids.sort_unstable();

    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ObjDeptError::Invalid {
            message: "duplicate object slot id".into(),
        });
    }

    oper.specs
        .iter()
        .map(|spec| {
            let single_oper = GenObjSlot {
                spec,
                _m: std::marker::PhantomData::<fn() -> ()>,
            };
            let slot = gen_slot(objs, tasks, topic, namespace, &single_oper)?;

            Ok((spec.id.to_owned(), slot))
        })
        .collect()
}

pub fn retire_objs<B>(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    oper: &RetireObjs<'_, B>,
) {
    let ids = match oper {
        RetireObjs::PreserveWatermarks { ids, .. }
        | RetireObjs::RemoveRows { ids, .. } => ids,
    };

    for id in *ids {
        let Some(record) = objs.get_mut(id) else {
            continue;
        };

        if let Some(meta) = record.meta.take() {
            tasks.push((topic, ObjTask::Delete { key: meta.key }));
        }
    }

    if matches!(oper, RetireObjs::RemoveRows { .. }) {
        for id in *ids {
            objs.remove(id);
        }
    }
}

#[macro_export]
macro_rules! implement_mock_obj_dept {
    ($obj:ty, $topic:literal, $namespace:literal, $url_profile:ident) => {
        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome,
            > {
                let mut state = self.state.lock().unwrap();
                let Some(record) = state
                    .objs
                    .get_mut($topic)
                    .and_then(|objs| objs.get_mut(&oper.key.id))
                else {
                    return Ok(
                        ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome::NotCurrent,
                    );
                };

                if record.version != oper.key.version {
                    return Ok(
                        ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome::NotCurrent,
                    );
                }

                let Some(meta) = record.meta.as_mut() else {
                    return Ok(
                        ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome::NotCurrent,
                    );
                };

                meta.is_available = true;

                Ok(
                    ::poprako_obj_dept::model::mark::MarkObjUploadedOutcome::Marked,
                )
            }
        }

        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                let state = self.state.lock().unwrap();

                Ok(oper
                    .ids
                    .iter()
                    .filter_map(|id| {
                        state
                            .objs
                            .get($topic)
                            .and_then(|objs| objs.get(id))
                            .and_then(|record| record.meta.clone())
                            .map(|obj_meta| (id.clone(), obj_meta))
                    })
                    .collect())
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                Ok(oper
                    .ids
                    .iter()
                    .filter_map(|id| {
                        context
                            .state
                            .objs
                            .get($topic)
                            .and_then(|objs| objs.get(id))
                            .and_then(|record| record.meta.clone())
                            .map(|obj_meta| (id.clone(), obj_meta))
                    })
                    .collect())
            }
        }

        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::GenObjUrls<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GenObjUrls<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    ::std::string::String,
                    ::poprako_obj_dept::model::url::ObjUrls,
                >,
            > {
                let mut urls = ::std::collections::HashMap::new();
                let thumbnail_enabled =
                    !self.flags.lock().unwrap().obj_thumbnail_disabled;

                for (id, obj_meta) in oper.metas {
                    let Some(url) =
                        $crate::part_impl::obj_dept::mock_impl::gen_urls(
                            $namespace,
                            Some(obj_meta),
                            ::poprako_obj_dept::pool::ObjUrlProfile::$url_profile,
                            thumbnail_enabled,
                        )?
                    else {
                        continue;
                    };

                    urls.insert(id.clone(), url);
                }

                Ok(urls)
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::GenObjSlots<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::GenObjSlots<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::slot::ObjSlot,
                >,
            > {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::gen_slots(
                    objs, obj_tasks, $topic, $namespace, oper,
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
                ::poprako_obj_dept::oper::RetireObjs<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::RetireObjs<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::retire_objs(
                    objs, obj_tasks, $topic, oper,
                );

                Ok(())
            }
        }
    };
}
