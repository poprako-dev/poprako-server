//! Typed fixture identities shared by object contract tests.

use serde::{Deserialize, Serialize};

use std::borrow::Borrow;

use poprako_obj_dept::key::{ObjId, ObjKind, ObjPath};
use poprako_obj_dept::rest::ObjDeptRest;

/// Fixture scalar retaining ordinary text for deliberately small contract tests.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Text(pub String);

impl ObjId for Text {
    fn from_text(value: &str) -> ObjDeptRest<Self> {
        Ok(Self(value.into()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<Text> for String {
    fn from(value: Text) -> Self {
        value.0
    }
}

/// Fixture domain input for one-way generation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSourceImageKey {
    /// Fixture identity.
    pub id: Text,
    /// Fixture suffix.
    pub ext: Text,
}

/// Fixture object marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageSourceImage;

impl ObjKind for PageSourceImage {
    type Id = Text;
    type Ext = Text;
    type Dom = PageSourceImageKey;

    fn id(value: &Self::Dom) -> &Self::Id {
        &value.id
    }

    fn ext(value: &Self::Dom) -> &Self::Ext {
        &value.ext
    }

    fn forward(value: &Self::Dom, ver: u32) -> ObjPath {
        ObjPath::new(format!(
            "page/{}-{}.{}",
            value.id.as_str(),
            ver,
            value.ext.as_str()
        ))
    }
}
