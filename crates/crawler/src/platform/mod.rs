pub mod atcoder;
pub mod codeforces;
pub mod leetcode;
pub mod luogu;
pub mod nowcoder;
pub mod time_util;

use async_trait::async_trait;

use crate::error::Result;
use crate::models::Competition;

#[async_trait]
pub trait ContestFetcher: Send + Sync {
    fn platform_name(&self) -> &str;
    async fn fetch(&self) -> Result<Vec<Competition>>;
}
