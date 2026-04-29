use serde::Deserialize;

use crate::error::{CrawlerError, Result};
use crate::models::BiliInfo;

#[derive(Deserialize)]
struct BiliApiResponse {
    code: i32,
    data: Option<BiliVideoData>,
}

#[derive(Deserialize)]
struct BiliVideoData {
    title: String,
    desc: String,
    pic: String,
    duration: u64,
    pubdate: i64,
    cid: i64,
    owner: BiliOwner,
    stat: BiliStat,
}

#[derive(Deserialize)]
struct BiliOwner {
    name: String,
}

#[derive(Deserialize)]
struct BiliStat {
    view: u64,
    like: u64,
    coin: u64,
    favorite: u64,
}

/// Fetch Bilibili video info by BV number
pub async fn get_bilibili_info(bv: &str) -> Result<BiliInfo> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .referer(true)
        .build()?;

    let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bv);
    let resp: BiliApiResponse = client.get(&url).send().await?.json().await?;

    parse_bilibili_response(&resp)
}

fn parse_bilibili_response(resp: &BiliApiResponse) -> Result<BiliInfo> {
    if resp.code != 0 {
        return Err(CrawlerError::Parse(format!(
            "Failed to get video info: code={}",
            resp.code
        )));
    }

    let data = resp
        .data
        .as_ref()
        .ok_or_else(|| CrawlerError::Parse("Empty data in response".into()))?;

    let duration_str = format_duration(data.duration);
    let publish_date = format_pubdate(data.pubdate);

    Ok(BiliInfo {
        title: data.title.clone(),
        author: data.owner.name.clone(),
        cover_url: data.pic.clone(),
        description: data.desc.clone(),
        video_url: None,
        duration: duration_str,
        play_count: data.stat.view,
        like_count: data.stat.like,
        coin_count: data.stat.coin,
        fav_count: data.stat.favorite,
        publish_date,
        cid: data.cid,
    })
}

fn format_pubdate(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| {
            let utc8 = dt.with_timezone(&crate::UTC8);
            utc8.format("%Y-%m-%d").to_string()
        })
        .unwrap_or_else(|| "未知".to_owned())
}

fn format_duration(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_short_duration() {
        assert_eq!(format_duration(65), "01:05");
        assert_eq!(format_duration(3661), "1:01:01");
    }

    #[test]
    fn format_long_duration() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(3600), "1:00:00");
    }

    #[test]
    fn parse_valid_response() {
        let resp: BiliApiResponse = serde_json::from_value(json!({
            "code": 0,
            "data": {
                "title": "Test Video",
                "desc": "A test description",
                "pic": "https://example.com/cover.jpg",
                "duration": 120,
                "pubdate": 1700000000,
                "cid": 123456789,
                "owner": { "name": "TestAuthor" },
                "stat": { "view": 1000, "like": 50, "coin": 30, "favorite": 25 }
            }
        }))
        .unwrap();

        let info = parse_bilibili_response(&resp).unwrap();
        assert_eq!(info.title, "Test Video");
        assert_eq!(info.author, "TestAuthor");
        assert_eq!(info.duration, "02:00");
        assert_eq!(info.play_count, 1000);
        assert_eq!(info.like_count, 50);
        assert_eq!(info.coin_count, 30);
        assert_eq!(info.fav_count, 25);
        assert_eq!(info.cid, 123456789);
        assert!(!info.publish_date.is_empty());
    }

    #[test]
    fn parse_error_code() {
        let resp: BiliApiResponse = serde_json::from_value(json!({
            "code": -404,
            "data": null
        }))
        .unwrap();

        assert!(parse_bilibili_response(&resp).is_err());
    }

    #[test]
    fn parse_null_data() {
        let resp: BiliApiResponse = serde_json::from_value(json!({
            "code": 0,
            "data": null
        }))
        .unwrap();

        assert!(parse_bilibili_response(&resp).is_err());
    }
}
