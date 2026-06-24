use uuid::Uuid;

pub struct ComicComplex;

impl ComicComplex {
    pub fn gen_id() -> String {
        format!("comic-{}", Uuid::now_v7())
    }

    pub fn gen_cover_key(id: &str, cover_version: i64, file_ext: &str) -> String {
        format!("comic_cover/{}-{}.{}", id, cover_version, file_ext)
    }
}
