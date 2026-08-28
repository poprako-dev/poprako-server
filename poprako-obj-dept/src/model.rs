use url::Url;

use crate::key::ObjKeyRef;

pub struct ObjSpec<'a> {
    pub key: ObjKeyRef<'a>,
}

pub struct ObjSlot {
    pub url: Url,
}
