pub mod bilibili;
pub mod bilibili_video;
pub mod error;
pub mod models;
pub mod platform;

use error::Result;
use models::Competition;
use platform::ContestFetcher;

use platform::atcoder::AtCoder;
use platform::codeforces::Codeforces;
use platform::leetcode::LeetCode;
use platform::luogu::Luogu;
use platform::nowcoder::NowCoder;

/// UTC+8 (China Standard Time) fixed offset.
pub const UTC8: chrono::FixedOffset = match chrono::FixedOffset::east_opt(8 * 3600) {
    Some(o) => o,
    None => panic!("UTC+8 is always valid"),
};

pub async fn get_all_competitions() -> Result<Vec<Competition>> {
    let fetchers: Vec<Box<dyn ContestFetcher>> = vec![
        Box::new(LeetCode),
        Box::new(Codeforces),
        Box::new(NowCoder),
        Box::new(AtCoder),
        Box::new(Luogu),
    ];

    let futures: Vec<_> = fetchers.iter().map(|f| f.fetch()).collect();
    let results = futures::future::join_all(futures).await;

    let mut all = Vec::new();
    for result in results {
        match result {
            Ok(contests) => all.extend(contests),
            Err(e) => log::warn!("Crawler error: {}", e),
        }
    }

    all.sort_by_key(|c| c.start_time);
    Ok(all)
}
