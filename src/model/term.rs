/// A single term entry carrying source text, translated target, and optional note.
pub struct TermInfo {
    pub id: String,

    pub source: String,
    pub target: String,

    pub note: Option<String>,
}
