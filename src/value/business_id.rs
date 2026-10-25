//! Canonical identities for persisted business entities.

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
