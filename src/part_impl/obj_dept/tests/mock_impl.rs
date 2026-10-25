//! In-memory `ObjDept` operations used by server tests.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};

use time::{Duration, OffsetDateTime};
use url::Url;

use poprako_obj_dept::key::{ObjId, ObjKey, ObjKind, ObjRegistration};
use poprako_obj_dept::model::alloc::ObjAlloc;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::record::ObjMetaRecord;
use poprako_obj_dept::model::slot::ObjSlot;
use poprako_obj_dept::model::task::ObjTask;
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{GenObjSlot, GenObjSlots};
use poprako_obj_dept::pool::ensure_url_spec;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};

/// Executes the typed object operation against fixture state.
///
/// # Errors
/// Returns an error for invalid fixture data or an invalid operation.
pub fn gen_urls<K: ObjKind>(
    meta: Option<&ObjMeta<K>>,
    spec: ObjUrlSpec,
    thumbnail_enabled: bool,
) -> ObjDeptRest<Option<ObjUrls>> {
    ensure_url_spec(spec)?;

    let Some(meta) = meta else {
        return Ok(None);
    };

    if !meta.is_avail {
        return Ok(None);
    }

    let key = meta.key.image().as_str();

    let origin_url = spec
        .includes_origin()
        .then(|| parse_mock_url(key))
        .transpose()?;

    let optimized_url = spec
        .includes_optimized()
        .then(|| parse_mock_url(&["optimized/", key].concat()))
        .transpose()?;

    let thumbnail_url = match (spec.includes_thumbnail(), thumbnail_enabled) {
        (true, true) => Some(parse_mock_url(&["thumbnail/", key].concat())?),
        (true, false) | (false, _) => None,
    };

    Ok(Some(ObjUrls {
        origin_url,
        optimized_url,
        thumbnail_url,
    }))
}

// Parses one deterministic object URL for tests.
fn parse_mock_url(path: &str) -> ObjDeptRest<Url> {
    Url::parse(&["https://obj.test/", path].concat()).map_err(|source| {
        ObjDeptError::Unrecoverable {
            message: source.to_string(),
        }
    })
}

/// Executes the typed object operation against fixture state.
///
/// # Errors
/// Returns an error for invalid fixture data or an invalid operation.
pub fn gen_slot<K: ObjRegistration>(
    objs: &mut HashMap<String, MockObjRecord, impl std::hash::BuildHasher>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    oper: &GenObjSlot<'_, K>,
) -> ObjDeptRest<ObjAlloc<K>>
where
    K::Id: Clone,
{
    let id = K::id(&oper.spec.dom);

    let matching_meta = objs
        .get(id.as_str())
        .and_then(|record| record.meta.as_ref())
        .filter(|meta| {
            meta.hash == oper.spec.hash
                && meta.ext == K::ext(&oper.spec.dom).as_str()
        });

    if let Some(meta) = matching_meta.filter(|meta| meta.is_avail) {
        return Ok(ObjAlloc::Avail {
            key: ObjKey::from_record(meta.key.clone())?,
        });
    }

    let reused_key = matching_meta.map(|meta| meta.key.clone());

    let (key, previous) = reused_key.map_or_else(
        || {
            let ver = objs.get(id.as_str()).map_or(Ok(1), |previous| {
                previous.version.checked_add(1).ok_or_else(|| {
                    ObjDeptError::Unrecoverable {
                        message: "object ver overflow".into(),
                    }
                })
            })?;

            let key = ObjKey::<K>::new(&oper.spec.dom, ver)?;

            let meta = ObjMetaRecord {
                key: key.to_record(),
                is_avail: false,
                hash: oper.spec.hash.to_vec(),
                ext: K::ext(&oper.spec.dom).as_str().into(),
            };

            let previous = objs.insert(
                id.as_str().into(),
                MockObjRecord {
                    version: ver,
                    meta: Some(meta),
                },
            );

            Ok::<_, ObjDeptError>((key, previous))
        },
        |key| ObjKey::<K>::from_record(key).map(|key| (key, None)),
    )?;

    if let Some(previous_key) =
        previous.and_then(|record| record.meta.map(|meta| meta.key))
    {
        tasks.push((K::TOPIC, ObjTask::Delete { key: previous_key }));
    }

    tasks.push((
        K::TOPIC,
        ObjTask::Check {
            key: key.to_record(),
        },
    ));

    let url = parse_mock_url(&format!("write/{}", key.image().as_str()))?;

    Ok(ObjAlloc::Upload {
        slot: ObjSlot {
            key,
            url,
            headers: BTreeMap::default(),
            expires_at: OffsetDateTime::now_utc() + Duration::minutes(5),
        },
    })
}

/// Executes the typed object operation against fixture state.
///
/// # Errors
/// Returns an error for invalid fixture data or an invalid operation.
pub fn gen_slots<K: ObjRegistration>(
    objs: &mut HashMap<String, MockObjRecord, impl std::hash::BuildHasher>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    oper: &GenObjSlots<'_, K>,
) -> ObjDeptRest<HashMap<K::Id, ObjAlloc<K>>>
where
    K::Id: Clone,
{
    let mut ids = oper
        .specs
        .iter()
        .map(|spec| K::id(&spec.dom))
        .collect::<Vec<_>>();

    ids.sort_unstable();

    if ids
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left == right))
    {
        return Err(ObjDeptError::invalid("duplicate object slot id".into()));
    }

    oper.specs
        .iter()
        .map(|spec| {
            let single_oper = GenObjSlot::<K>::new(spec);

            let alloc = gen_slot(objs, tasks, &single_oper)?;

            Ok((K::id(&spec.dom).clone(), alloc))
        })
        .collect()
}

pub fn clear_objs<K: ObjRegistration>(
    objs: &mut HashMap<String, MockObjRecord, impl std::hash::BuildHasher>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    ids: &[K::Id],
) {
    for id in ids {
        let Some(record) = objs.get_mut(id.as_str()) else {
            continue;
        };

        if let Some(meta) = record.meta.take() {
            tasks.push((K::TOPIC, ObjTask::Delete { key: meta.key }));
        }
    }
}

pub fn delete_objs<K: ObjRegistration>(
    objs: &mut HashMap<String, MockObjRecord, impl std::hash::BuildHasher>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    ids: &[K::Id],
) {
    for id in ids {
        let Some(record) = objs.remove(id.as_str()) else {
            continue;
        };

        if let Some(meta) = record.meta {
            tasks.push((K::TOPIC, ObjTask::Delete { key: meta.key }));
        }
    }
}

#[macro_export]
macro_rules! implement_mock_obj_dept {
    ($obj:ty, $topic:literal) => {
        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                let mut state = self.state.lock().unwrap();
                let Some(record) = state
                    .objs
                    .get_mut($topic)
                    .and_then(|objs| objs.get_mut(oper.key.id().as_str()))
                else {
                    return Ok(false);
                };

                if record.version != oper.key.ver() {
                    return Ok(false);
                }

                let Some(meta) = record.meta.as_mut() else {
                    return Ok(false);
                };

                meta.is_avail = true;

                drop(state);

                Ok(true)
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                let state = &mut context.state;
                let Some(record) = state
                    .objs
                    .get_mut($topic)
                    .and_then(|objs| objs.get_mut(oper.key.id().as_str()))
                else {
                    return Ok(false);
                };

                if record.version != oper.key.ver() {
                    return Ok(false);
                }

                let Some(meta) = record.meta.as_mut() else {
                    return Ok(false);
                };

                meta.is_avail = true;

                Ok(true)
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
                    <$obj as ::poprako_obj_dept::key::ObjKind>::Id,
                    ::poprako_obj_dept::model::meta::ObjMeta<$obj>,
                >,
            > {
                let state = self.state.lock().unwrap();

                oper.ids
                    .iter()
                    .cloned()
                    .filter_map(|id| {
                        state
                            .objs
                            .get($topic)
                            .and_then(|objs| objs.get(id.as_str()))
                            .and_then(|record| record.meta.clone())
                            .map(|obj_meta| {
                                obj_meta.decode::<$obj>().map(|meta| (id, meta))
                            })
                    })
                    .collect::<::poprako_obj_dept::rest::ObjDeptRest<_>>()
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
                    <$obj as ::poprako_obj_dept::key::ObjKind>::Id,
                    ::poprako_obj_dept::model::meta::ObjMeta<$obj>,
                >,
            > {
                oper.ids
                    .iter()
                    .cloned()
                    .filter_map(|id| {
                        context
                            .state
                            .objs
                            .get($topic)
                            .and_then(|objs| objs.get(id.as_str()))
                            .and_then(|record| record.meta.clone())
                            .map(|obj_meta| {
                                obj_meta.decode::<$obj>().map(|meta| (id, meta))
                            })
                    })
                    .collect::<::poprako_obj_dept::rest::ObjDeptRest<_>>()
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
                    <$obj as ::poprako_obj_dept::key::ObjKind>::Id,
                    ::poprako_obj_dept::model::url::ObjUrls,
                >,
            > {
                ::poprako_obj_dept::pool::ensure_url_spec(oper.spec)?;

                let mut urls = ::std::collections::HashMap::new();
                let thumbnail_enabled =
                    !self.flags.lock().unwrap().obj_thumbnail_disabled;

                for (id, obj_meta) in oper.metas {
                    let Some(url) =
                        $crate::part_impl::obj_dept::tests::mock_impl::gen_urls(
                            Some(obj_meta),
                            oper.spec,
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
                    <$obj as ::poprako_obj_dept::key::ObjKind>::Id,
                    ::poprako_obj_dept::model::alloc::ObjAlloc<$obj>,
                >,
            > {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::tests::mock_impl::gen_slots(
                    objs, obj_tasks, oper,
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
                ::poprako_obj_dept::model::alloc::ObjAlloc<$obj>,
            > {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::tests::mock_impl::gen_slot(
                    objs, obj_tasks, oper,
                )
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::ClearObjs<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::ClearObjs<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::tests::mock_impl::clear_objs::<
                    $obj,
                >(
                    objs, obj_tasks, oper.ids,
                );

                Ok(())
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::DeleteObjs<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::DeleteObjs<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                if context.obj_delete_failure {
                    return Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                            message: "mock object deletion failed".into(),
                        },
                    );
                }

                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::tests::mock_impl::delete_objs::<
                    $obj,
                >(
                    objs, obj_tasks, oper.ids,
                );

                Ok(())
            }
        }
    };
}
