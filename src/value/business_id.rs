//! Canonical identities for persisted business entities.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Closed registry of business identity namespaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusinessIdKind {
    /// The user identity namespace.
    User,

    /// The team identity namespace.
    Team,

    /// The member identity namespace.
    Member,

    /// The member invitation identity namespace.
    MemberInvitation,

    /// The workset identity namespace.
    Workset,

    /// The comic identity namespace.
    Comic,

    /// The chapter identity namespace.
    Chapter,

    /// The page source identity namespace.
    PageSource,

    /// The page artwork identity namespace.
    PageArtwork,

    /// The unit identity namespace.
    Unit,

    /// The issue identity namespace.
    Issue,

    /// The assignment identity namespace.
    Assignment,

    /// The assignment invitation identity namespace.
    AssignmentInvitation,

    /// The announcement identity namespace.
    Announcement,

    /// The comment identity namespace.
    Comment,

    /// The comic archive identity namespace.
    ComicArchive,

    /// The termbase identity namespace.
    Termbase,

    /// The term identity namespace.
    Term,

    /// The system mail identity namespace.
    SystemMail,

    /// The chapter workflow record identity namespace.
    ChapterWorkflowRecord,
}

impl BusinessIdKind {
    /// Returns the canonical lowercase domain prefix.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        //
        match self {
            //
            Self::User => "user",

            Self::Team => "team",

            Self::Member => "member",

            Self::MemberInvitation => "member_invitation",

            Self::Workset => "workset",

            Self::Comic => "comic",

            Self::Chapter => "chapter",

            Self::PageSource => "page_source",

            Self::PageArtwork => "page_artwork",

            Self::Unit => "unit",

            Self::Issue => "issue",

            Self::Assignment => "assignment",

            Self::AssignmentInvitation => "assignment_invitation",

            Self::Announcement => "announcement",

            Self::Comment => "comment",

            Self::ComicArchive => "comic_archive",

            Self::Termbase => "termbase",

            Self::Term => "term",

            Self::SystemMail => "system_mail",

            Self::ChapterWorkflowRecord => "chapter_workflow_record",
        }
    }

    /// Validates the exact namespace and canonical eleven-character Base62 suffix.
    #[must_use]
    pub fn is_valid_id(self, value: &str) -> bool {
        //
        let Some((prefix, suffix)) = value.split_once("--") else {
            return false;
        };

        if prefix != self.prefix() || suffix.len() != 11 {
            return false;
        }

        let Ok(number) = base62::decode(suffix) else {
            return false;
        };

        let Ok(number) = u64::try_from(number) else {
            return false;
        };

        suffix == format!("{:0>11}", base62::encode(number))
    }
}

// Defines validated object-facing identities without changing business String storage.
macro_rules! business_identity {
    ($name:ident, $kind:ident) => {
        /// Validated canonical business identity for object operations.
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Validates the namespace and canonical identifier encoding.
            ///
            /// # Errors
            /// Returns an error for a wrong namespace or noncanonical encoding.
            pub fn new(
                value: &str,
            ) -> poprako_obj_dept::rest::ObjDeptRest<Self> {
                <Self as poprako_obj_dept::key::ObjId>::from_owned(value.into())
            }

            /// Borrows the canonical identifier.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl poprako_obj_dept::key::ObjId for $name {
            fn from_text(
                value: &str,
            ) -> poprako_obj_dept::rest::ObjDeptRest<Self> {
                Self::new(value)
            }

            fn from_owned(
                value: String,
            ) -> poprako_obj_dept::rest::ObjDeptRest<Self> {
                if !BusinessIdKind::$kind.is_valid_id(&value) {
                    return Err(poprako_obj_dept::rest::ObjDeptError::invalid(
                        concat!("invalid ", stringify!($kind), " identity")
                            .into(),
                    ));
                }

                Ok(Self(value))
            }

            fn as_str(&self) -> &str {
                self.as_str()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;

                <Self as poprako_obj_dept::key::ObjId>::from_owned(value)
                    .map_err(D::Error::custom)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }
    };
}

business_identity!(UserId, User);

business_identity!(TeamId, Team);

business_identity!(ComicId, Comic);

business_identity!(ChapterId, Chapter);

business_identity!(PageSourceId, PageSource);

business_identity!(PageArtworkId, PageArtwork);
