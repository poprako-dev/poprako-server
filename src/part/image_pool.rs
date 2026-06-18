use async_trait::async_trait;
use url::Url;

use crate::result::RootResult;

#[async_trait]
pub trait ImagePool {
    async fn get_signed(&self, key: &str) -> RootResult<Url>;
}
