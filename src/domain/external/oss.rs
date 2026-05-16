use url::Url;

pub trait OssGetSigner {
    fn sign_get(&self, key: &str) -> Option<Url>;
}
